use std::{fmt::Debug, sync::Arc};

use diagnostic::{miette::Diagnostic, Dr};
use files::{Path, Source, SourceData, SourceSpan, Span, Str, WithProvenance};
use kernel::{
    expr::{
        ArgumentStyle, Binder, BinderStructure, BoundVariable, Expression, InvocationStyle,
        Universe, Usage,
    },
    DeBruijnIndex,
};
use thiserror::Error;
use tree_sitter::{Node, TreeCursor};
use upcast::Upcast;

pub type ParseDr<T> = Dr<T, ParseError, ParseError>;

#[salsa::jar(db = Db)]
pub struct Jar(parse_module);

pub trait Db: kernel::Db + Upcast<dyn kernel::Db> + salsa::DbWithJar<Jar> {}

impl<T> Db for T where T: kernel::Db + salsa::DbWithJar<Jar> + 'static {}

#[tracing::instrument(level = "debug")]
#[salsa::tracked]
pub fn parse_module(db: &dyn Db, source: Source) -> Dr<Module, ParseError, ParseError> {
    files::source(db.up(), source)
        .map_err(|_| todo!())
        .map_errs(|_| todo!())
        .bind(|code| {
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(tree_sitter_feather::language())
                .expect("Error loading Feather grammar");
            let tree = parser.parse(&*code, None).unwrap();

            if tree.root_node().kind() != "source_file" {
                return Dr::new_err(ParseError::parser_bug(
                    db,
                    source,
                    "root node was not `source_file`",
                ));
            }

            tracing::trace!("{}", tree.root_node().to_sexp());

            let mut errors = Vec::new();
            check_errors(db, source, &mut tree.root_node().walk(), &mut errors);
            if !errors.is_empty() {
                return Dr::new_err_many(errors);
            }

            process_module(db, source, &code, tree.root_node())
        })
}

/// Search through the node tree given by `cursor` for any error notes, and add them to `errors`.
/// This function provides pretty poor error messages, but it's good enough for now.
/// Later, we can use contextual information (such as where an error node is positioned in the tree)
/// to give better diagnostics, and provide suggestions.
fn check_errors(
    db: &dyn Db,
    source: Source,
    cursor: &mut TreeCursor,
    errors: &mut Vec<ParseError>,
) {
    if cursor.node().is_error() {
        errors.push(ParseError::ParseError {
            src: source.data(db.up()),
            label_span: cursor.node().byte_range().into(),
        });
    } else if cursor.goto_first_child() {
        loop {
            check_errors(db, source, cursor, errors);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module {
    path: WithProvenance<Path>,
    definitions: Vec<WithProvenance<Definition>>,
}

/// Converts a parsed node into a [`Module`].
/// We assume that there were no syntax errors.
fn process_module(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    root_node: Node,
) -> ParseDr<Module> {
    assert_eq!(root_node.kind(), "source_file");
    // Process the module's name.
    let path = process_path(
        db,
        source,
        code,
        root_node
            .child_by_field_name("module")
            .unwrap()
            .child_by_field_name("path")
            .unwrap(),
    );

    // Process all of the definitions.
    let definitions = Dr::sequence_unfail(
        root_node
            .children_by_field_name("definition", &mut root_node.walk())
            .map(|node| process_definition(db, source, code, node)),
    );

    definitions.map(|definitions| Module { path, definitions })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    name: WithProvenance<Str>,
    usage: Usage,
    ty: Expression,
    body: Expression,
}

fn process_definition(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
) -> ParseDr<WithProvenance<Definition>> {
    assert_eq!(node.kind(), "definition");
    let name = node.child_by_field_name("name").unwrap();
    let erased = node.child_by_field_name("usage").is_some();
    let ty = node.child_by_field_name("ty").unwrap();
    let body = node.child_by_field_name("body").unwrap();
    process_expr(db, source, code, ty, &[]).bind(|ty| {
        process_expr(db, source, code, body, &[]).map(|body| {
            WithProvenance::new(
                Some(SourceSpan::new(source, node.byte_range().into())),
                Definition {
                    name: process_identifier(db, source, code, name),
                    usage: if erased {
                        Usage::Erased
                    } else {
                        Usage::Present
                    },
                    ty,
                    body,
                },
            )
        })
    })
}

fn process_path(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
) -> WithProvenance<Path> {
    let segments = node
        .children_by_field_name("first", &mut node.walk())
        .chain(std::iter::once(node.child_by_field_name("last").unwrap()))
        .map(|node| Str::new(db.up(), node.utf8_text(code.as_bytes()).unwrap().to_owned()))
        .collect::<Vec<_>>();
    WithProvenance::new(
        Some(SourceSpan::new(source, node.byte_range().into())),
        Path::new(db.up(), segments),
    )
}

fn process_identifier(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
) -> WithProvenance<Str> {
    assert_eq!(node.kind(), "identifier");
    WithProvenance::new(
        Some(SourceSpan::new(source, node.byte_range().into())),
        Str::new(db.up(), node.utf8_text(code.as_bytes()).unwrap().to_owned()),
    )
}

fn process_universe(source: Source, code: &Arc<String>, node: Node) -> WithProvenance<Universe> {
    assert_eq!(node.kind(), "universe");
    WithProvenance::new(
        Some(SourceSpan::new(source, node.byte_range().into())),
        Universe(
            node.utf8_text(code.as_bytes())
                .unwrap()
                .parse()
                .expect("did not fit into a u32"),
        ),
    )
}

fn process_expr(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    match node.kind() {
        "paren" => process_expr(
            db,
            source,
            code,
            node.child_by_field_name("inner").unwrap(),
            locals,
        ),
        "local" => process_local(db, source, code, node, locals),
        "app" => process_app(db, source, code, node, locals),
        "for" => process_for(db, source, code, node, locals),
        "fun" => process_fun(db, source, code, node, locals),
        "let" => process_let(db, source, code, node, locals),
        "sort" => Dr::new(process_sort(db, source, code, node)),
        "inst" => Dr::new(process_inst(db, source, code, node)),
        "intro" => process_intro(db, source, code, node, locals),
        "match" => process_match(db, source, code, node, locals),
        "fix" => process_fix(db, source, code, node, locals),
        "ref" => process_ref(db, source, code, node, locals),
        "deref" => process_deref(db, source, code, node, locals),
        "loan" => process_loan(db, source, code, node, locals),
        "take" => process_take(db, source, code, node, locals),
        "in" => process_in(db, source, code, node, locals),
        value => todo!("{value}"),
    }
}

fn process_de_bruijn_index(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<DeBruijnIndex> {
    let name = Str::new(db.up(), node.utf8_text(code.as_bytes()).unwrap().to_owned());
    if let Some(index) = locals.iter().position(|value| *value == name) {
        Dr::new(DeBruijnIndex::new(index as u32))
    } else {
        Dr::new(DeBruijnIndex::zero()).with(ParseError::UnknownVariable {
            src: source.data(db.up()),
            label_span: node.byte_range().into(),
        })
    }
}

fn process_local(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "local");
    process_de_bruijn_index(db, source, code, node, locals)
        .map(|index| Expression::new_local(db.up(), index))
}

fn process_app(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "app");
    process_expr(
        db,
        source,
        code,
        node.child_by_field_name("left").unwrap(),
        locals,
    )
    .bind(|left| {
        process_expr(
            db,
            source,
            code,
            node.child_by_field_name("right").unwrap(),
            locals,
        )
        .map(|right| Expression::new_apply(db.up(), left, right))
    })
}

fn process_binder_structure(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
    invocation_style: InvocationStyle,
) -> ParseDr<BinderStructure> {
    let name = process_identifier(db, source, code, node.child_by_field_name("name").unwrap());
    let erased = node.child_by_field_name("usage").is_some();
    let ty = node.child_by_field_name("ty").unwrap();
    process_expr(db, source, code, ty, locals).map(|ty| BinderStructure {
        bound: BoundVariable {
            name: name.contents,
            ty,
            usage: if erased {
                Usage::Erased
            } else {
                Usage::Present
            },
        },
        argument_style: match node.kind() {
            "explicit" => ArgumentStyle::Explicit,
            "implicit" => ArgumentStyle::ImplicitEager,
            "weak" => ArgumentStyle::ImplicitWeak,
            _ => unreachable!(),
        },
        invocation_style,
    })
}

fn process_binder(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Binder> {
    let binder_structure = node.child_by_field_name("binder_structure").unwrap();
    let arrow = node.child_by_field_name("arrow").unwrap();
    let body = node.child_by_field_name("body").unwrap();
    process_binder_structure(
        db,
        source,
        code,
        binder_structure,
        locals,
        match arrow.utf8_text(code.as_bytes()).unwrap() {
            "->" => InvocationStyle::Once,
            "=>" => InvocationStyle::Many,
            _ => unreachable!(),
        },
    )
    .bind(|structure| {
        let new_locals = std::iter::once(structure.bound.name)
            .chain(locals.iter().copied())
            .collect::<Vec<_>>();
        process_expr(db, source, code, body, &new_locals).map(|body| Binder { structure, body })
    })
}

fn process_for(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "for");
    process_binder(db, source, code, node, locals).map(|binder| Expression::new_pi(db.up(), binder))
}

fn process_fun(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "fun");
    process_binder(db, source, code, node, locals)
        .map(|binder| Expression::new_lambda(db.up(), binder))
}

fn process_let(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "let");
    let name = process_identifier(db, source, code, node.child_by_field_name("name").unwrap());
    let to_assign = process_expr(
        db,
        source,
        code,
        node.child_by_field_name("to_assign").unwrap(),
        locals,
    );
    let mut locals = locals.to_vec();
    locals.insert(0, name.contents);
    let body = process_expr(
        db,
        source,
        code,
        node.child_by_field_name("body").unwrap(),
        &locals,
    );
    to_assign.bind(|to_assign| {
        body.map(|body| Expression::new_let(db.up(), name.contents, to_assign, body))
    })
}

fn process_sort(db: &dyn Db, source: Source, code: &Arc<String>, node: Node) -> Expression {
    Expression::new_sort(
        db.up(),
        process_universe(source, code, node.child_by_field_name("universe").unwrap()).contents,
    )
}

fn process_inst(db: &dyn Db, source: Source, code: &Arc<String>, node: Node) -> Expression {
    assert_eq!(node.kind(), "inst");
    Expression::new_inst(
        db.up(),
        process_path(db, source, code, node.child_by_field_name("path").unwrap()).contents,
    )
}

fn process_intro(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "intro");
    let path = process_path(db, source, code, node.child_by_field_name("path").unwrap());
    let parameters = Dr::sequence_unfail(
        node.children_by_field_name("param", &mut node.walk())
            .map(|param| process_expr(db, source, code, param, locals)),
    );

    let variant = process_identifier(
        db,
        source,
        code,
        node.child_by_field_name("variant").unwrap(),
    );

    let fields = Dr::sequence_unfail(node.children_by_field_name("field", &mut node.walk()).map(
        |field| {
            assert_eq!(node.kind(), "intro_field");
            let name =
                process_identifier(db, source, code, field.child_by_field_name("name").unwrap());
            process_expr(
                db,
                source,
                code,
                node.child_by_field_name("value").unwrap(),
                locals,
            )
            .map(|value| (name.contents, value))
        },
    ));

    parameters.bind(|parameters| {
        fields.map(|fields| {
            Expression::new_intro(
                db.up(),
                path.contents,
                parameters,
                variant.contents,
                fields.into(),
            )
        })
    })
}

fn process_match(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "match");

    let subject = process_expr(
        db,
        source,
        code,
        node.child_by_field_name("subject").unwrap(),
        locals,
    );

    let return_ty = process_expr(
        db,
        source,
        code,
        node.child_by_field_name("return").unwrap(),
        locals,
    );

    let body = node.child_by_field_name("body").unwrap();
    let cases = Dr::sequence_unfail(
        body.children_by_field_name("variant", &mut body.walk())
            .map(|variant| {
                let name = process_identifier(
                    db,
                    source,
                    code,
                    variant.child_by_field_name("name").unwrap(),
                );
                process_expr(
                    db,
                    source,
                    code,
                    variant.child_by_field_name("value").unwrap(),
                    locals,
                )
                .map(|value| (name.contents, value))
            }),
    );

    subject.bind(|subject| {
        return_ty.bind(|return_ty| {
            cases.map(|cases| Expression::new_match(db.up(), subject, return_ty, cases.into()))
        })
    })
}

fn process_fix(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "fix");

    let binder_structure = process_binder_structure(
        db,
        source,
        code,
        node.child_by_field_name("binder_structure").unwrap(),
        locals,
        InvocationStyle::Many,
    );

    binder_structure.bind(|binder_structure| {
        let mut locals = locals.to_vec();
        locals.insert(0, binder_structure.bound.name);
        let return_ty = process_expr(
            db,
            source,
            code,
            node.child_by_field_name("return").unwrap(),
            &locals,
        );

        let rec_name = process_identifier(
            db,
            source,
            code,
            node.child_by_field_name("rec_name").unwrap(),
        );
        locals.insert(0, rec_name.contents);
        let body = process_expr(
            db,
            source,
            code,
            node.child_by_field_name("body").unwrap(),
            &locals,
        );

        return_ty.bind(|return_ty| {
            body.map(|body| {
                Expression::new_fix(
                    db.up(),
                    Binder {
                        structure: binder_structure,
                        body: return_ty,
                    },
                    rec_name.contents,
                    body,
                )
            })
        })
    })
}

fn process_ref(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    process_expr(
        db,
        source,
        code,
        node.child_by_field_name("ty").unwrap(),
        locals,
    )
    .map(|ty| Expression::new_ref(db.up(), ty))
}

fn process_deref(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    process_expr(
        db,
        source,
        code,
        node.child_by_field_name("value").unwrap(),
        locals,
    )
    .map(|ty| Expression::new_deref(db.up(), ty))
}

fn process_loan(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "loan");

    let local = process_de_bruijn_index(
        db,
        source,
        code,
        node.child_by_field_name("ident").unwrap(),
        locals,
    );
    let loan_as = process_identifier(db, source, code, node.child_by_field_name("as").unwrap());
    let with = process_identifier(db, source, code, node.child_by_field_name("with").unwrap());

    let mut locals = locals.to_vec();
    locals.insert(0, loan_as.contents);
    locals.insert(0, with.contents);
    let body = process_expr(
        db,
        source,
        code,
        node.child_by_field_name("body").unwrap(),
        &locals,
    );

    local.bind(|local| {
        body.map(|body| Expression::new_loan(db.up(), local, loan_as.contents, with.contents, body))
    })
}

fn process_take(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "take");

    let local = process_de_bruijn_index(
        db,
        source,
        code,
        node.child_by_field_name("ident").unwrap(),
        locals,
    );
    let proofs = Dr::sequence_unfail(node.children_by_field_name("proof", &mut node.walk()).map(
        |proof| {
            let local = process_de_bruijn_index(
                db,
                source,
                code,
                proof.child_by_field_name("local").unwrap(),
                locals,
            );
            let proof_term = process_expr(
                db,
                source,
                code,
                proof.child_by_field_name("proof").unwrap(),
                locals,
            );
            local.bind(|local| proof_term.map(|proof_term| (local, proof_term)))
        },
    ));
    let body = process_expr(
        db,
        source,
        code,
        node.child_by_field_name("body").unwrap(),
        locals,
    );

    local.bind(|local| {
        proofs.bind(|proofs| {
            body.map(|body| Expression::new_take(db.up(), local, proofs.into(), body))
        })
    })
}

fn process_in(
    db: &dyn Db,
    source: Source,
    code: &Arc<String>,
    node: Node,
    locals: &[Str],
) -> ParseDr<Expression> {
    assert_eq!(node.kind(), "in");
    process_expr(
        db,
        source,
        code,
        node.child_by_field_name("reference").unwrap(),
        locals,
    )
    .bind(|reference| {
        process_expr(
            db,
            source,
            code,
            node.child_by_field_name("target").unwrap(),
            locals,
        )
        .map(|target| Expression::new_in(db.up(), reference, target))
    })
}

#[derive(Error, Diagnostic, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseError {
    #[error("parser bug: {message}")]
    #[diagnostic(help = "this is a bug in the compiler")]
    ParserBug {
        #[source_code]
        src: SourceData,
        message: String,
        label_message: String,
        #[label("{label_message}")]
        label_span: Span,
    },
    #[error("syntax error")]
    ParseError {
        #[source_code]
        src: SourceData,
        #[label("error occurred here")]
        label_span: Span,
    },
    #[error("unknown local variable")]
    UnknownVariable {
        #[source_code]
        src: SourceData,
        #[label("error occurred here")]
        label_span: Span,
    },
}

impl ParseError {
    pub fn parser_bug(db: &dyn Db, source: Source, message: impl ToString) -> ParseError {
        ParseError::ParserBug {
            src: source.data(db.up()),
            message: message.to_string(),
            label_message: "error occurred here".to_owned(),
            label_span: Default::default(),
        }
    }
}
