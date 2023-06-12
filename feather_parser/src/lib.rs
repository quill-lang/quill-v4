use std::{fmt::Debug, sync::Arc};

use diagnostic::{miette::Diagnostic, Dr};
use files::{Db, Path, Source, SourceSpan, Span, Str, WithProvenance};
use thiserror::Error;
use tree_sitter::{Node, TreeCursor};

pub type ParseDr<T> = Dr<T, ParseError, ParseError>;

pub fn test(db: &dyn Db, source: Source, code: Arc<String>) -> Dr<Module, ParseError, ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_feather::language())
        .expect("Error loading feather grammar");
    let tree = parser.parse(&*code, None).unwrap();

    if tree.root_node().kind() != "source_file" {
        return Dr::new_err(ParseError::parser_bug(
            code,
            "root node was not `source_file`",
        ));
    }

    let mut cursor = tree.root_node().walk();
    let mut errors = Vec::new();
    check_errors(&code, &mut cursor, &mut errors);
    if !errors.is_empty() {
        return Dr::new_err_many(errors);
    }

    process_module(db, source, code, tree.root_node())
}

#[derive(Error, Diagnostic, Debug)]
pub enum ParseError {
    #[error("parser bug: {message}")]
    #[diagnostic(help = "this is a bug in the compiler")]
    ParserBug {
        #[source_code]
        src: Arc<String>,
        message: String,
        label_message: String,
        #[label("{label_message}")]
        label_span: Span,
    },
    #[error("syntax error")]
    ParseError {
        #[source_code]
        src: Arc<String>,
        #[label("error occurred here")]
        label_span: Span,
    },
}

impl ParseError {
    pub fn parser_bug(code: Arc<String>, message: impl ToString) -> ParseError {
        ParseError::ParserBug {
            src: code,
            message: message.to_string(),
            label_message: "error occurred here".to_owned(),
            label_span: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Module {
    name: Path,
}

/// Search through the node tree given by `cursor` for any error notes, and add them to `errors`.
/// This function provides pretty poor error messages, but it's good enough for now.
/// Later, we can use contextual information (such as where an error node is positioned in the tree)
/// to give better diagnostics, and provide suggestions.
fn check_errors(code: &Arc<String>, cursor: &mut TreeCursor, errors: &mut Vec<ParseError>) {
    if cursor.node().is_error() {
        errors.push(ParseError::ParseError {
            src: code.clone(),
            label_span: cursor.node().byte_range().into(),
        });
    } else if cursor.goto_first_child() {
        loop {
            check_errors(code, cursor, errors);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Converts a parsed node into a [`Module`].
/// We assume that there were no syntax errors.
fn process_module(
    db: &dyn Db,
    source: Source,
    code: Arc<String>,
    root_node: Node,
) -> ParseDr<Module> {
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
    println!("{}", path.contents.display(db));
    todo!()
}

fn process_path(
    db: &dyn Db,
    source: Source,
    code: Arc<String>,
    node: Node,
) -> WithProvenance<Path> {
    let mut cursor = node.walk();
    let segments = node
        .children_by_field_name("first", &mut cursor)
        .chain(std::iter::once(node.child_by_field_name("last").unwrap()))
        .map(|node| Str::new(db, node.utf8_text(code.as_bytes()).unwrap().to_owned()))
        .collect::<Vec<_>>();
    WithProvenance::new(
        Some(SourceSpan::new(source, node.byte_range().into())),
        Path::new(db, segments),
    )
}
