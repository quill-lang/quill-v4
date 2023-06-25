use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{Debug, Write},
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use files::{InputFile, Str};
use kernel::expr::{
    ArgumentStyle, Binder, BinderStructure, Expression, ExpressionData, InvocationStyle, Usage,
};
use notify_debouncer_mini::notify::RecursiveMode;
use salsa::Snapshot;

/// The main database that manages all the compiler's queries.
#[salsa::db(files::Jar, kernel::Jar, feather_parser::Jar)]
pub struct FeatherDatabase {
    storage: salsa::Storage<Self>,
    project_root: PathBuf,
    files: Arc<Mutex<HashMap<PathBuf, InputFile>>>,
    watcher: Arc<
        Mutex<notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>>,
    >,
}

impl Debug for FeatherDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<db>")
    }
}

impl salsa::Database for FeatherDatabase {}
impl salsa::ParallelDatabase for FeatherDatabase {
    fn snapshot(&self) -> Snapshot<Self> {
        Snapshot::new(FeatherDatabase {
            storage: self.storage.snapshot(),
            project_root: self.project_root.clone(),
            files: Arc::clone(&self.files),
            watcher: Arc::clone(&self.watcher),
        })
    }
}

impl files::Db for FeatherDatabase {
    fn input_file(&self, path: PathBuf) -> std::io::Result<InputFile> {
        let path = self.project_root.join(&path).canonicalize().map_err(|e| {
            std::io::Error::new(e.kind(), format!("failed to read {}", path.display()))
        })?;
        Ok(match self.files.lock().unwrap().entry(path.clone()) {
            // If the file already exists in our cache then just return it.
            Entry::Occupied(entry) => *entry.get(),
            // If we haven't read this file yet set up the watch, read the
            // contents, store it in the cache, and return it.
            Entry::Vacant(entry) => {
                // Set up the watch before reading the contents to try to avoid
                // race conditions.
                let watcher = &mut *self.watcher.lock().unwrap();
                watcher
                    .watcher()
                    .watch(&path, RecursiveMode::NonRecursive)
                    .unwrap();
                let contents = std::fs::read_to_string(&path).map_err(|e| {
                    std::io::Error::new(e.kind(), format!("failed to read {}", path.display()))
                })?;
                *entry.insert(InputFile::new(self, path, Arc::new(contents)))
            }
        })
    }
}

/// Internally used to implement [`kernel::Db::format_expression`].
/// Writes badly-formatted but clear and unambiguous Feather code representing the given expression.
/// This will then be run through the formatter.
/// TODO: Precedence levels (this function will currently produce some incorrect results).
fn write_expression(
    db: &FeatherDatabase,
    expr: Expression,
    locals: &[Str],
    w: &mut impl Write,
) -> std::fmt::Result {
    match expr.data(db) {
        ExpressionData::Local(index) => match locals.get(index.value() as usize) {
            Some(name) => {
                // TODO: Check if there is something with the same name at a lower index.
                write!(w, "{}", name.text(db))
            }
            None => write!(w, "<local {}>", index.value()),
        },
        ExpressionData::Apply { left, right } => {
            write_expression(db, left, locals, w)?;
            write!(w, " ( ")?;
            write_expression(db, right, locals, w)?;
            write!(w, " )")
        }
        ExpressionData::Lambda(binder) => {
            write!(w, "fun ")?;
            write_binder(db, binder, locals, w)
        }
        ExpressionData::Pi(binder) => {
            write!(w, "for ")?;
            write_binder(db, binder, locals, w)
        }
        ExpressionData::Let {
            name,
            to_assign,
            body,
        } => {
            write!(w, "let {} = ", name.text(db))?;
            write_expression(db, to_assign, locals, w)?;
            write!(w, " ;\n")?;
            let mut new_locals = locals.to_vec();
            new_locals.insert(0, name);
            write_expression(db, body, &new_locals, w)
        }
        ExpressionData::Sort(universe) => {
            write!(w, "Sort {}", universe.0)
        }
        ExpressionData::Inst(path) => {
            write!(w, "inst {}", path.display(db))
        }
        ExpressionData::Intro {
            path,
            parameters,
            variant,
            fields,
        } => {
            write!(w, "intro {}", path.display(db))?;
            for param in parameters {
                write!(w, " ( ")?;
                write_expression(db, param, locals, w)?;
                write!(w, " )")?;
            }
            write!(w, " / {} {{", variant.text(db))?;
            for (name, field) in fields.iter() {
                write!(w, "\n{} = ", name.text(db))?;
                write_expression(db, *field, locals, w)?;
                write!(w, " , ")?;
            }
            write!(w, "\n}}")
        }
        ExpressionData::Match {
            subject,
            return_ty,
            cases,
        } => {
            write!(w, "match ")?;
            write_expression(db, subject, locals, w)?;
            write!(w, " return ")?;
            write_expression(db, return_ty, locals, w)?;
            write!(w, " {{")?;
            for (name, case) in cases.iter() {
                write!(w, "\n{} -> ", name.text(db))?;
                write_expression(db, *case, locals, w)?;
                write!(w, " ,")?;
            }
            write!(w, "\n}}")
        }
        ExpressionData::Fix {
            binder,
            rec_name,
            body,
        } => {
            write!(w, "fix ")?;
            write_binder(db, binder, locals, w)?;
            write!(w, " with {} ; ", rec_name.text(db))?;
            let mut new_locals = locals.to_vec();
            new_locals.insert(0, binder.structure.bound.name);
            new_locals.insert(0, rec_name);
            write_expression(db, body, &new_locals, w)
        }
        ExpressionData::Ref(ty) => {
            write!(w, "ref ")?;
            write_expression(db, ty, locals, w)
        }
        ExpressionData::Deref(value) => {
            write!(w, "* ")?;
            write_expression(db, value, locals, w)
        }
        ExpressionData::Loan {
            local,
            loan_as,
            with,
            body,
        } => {
            let local = match locals.get(local.value() as usize) {
                Some(local) => local.text(db).clone(),
                None => format!("<local {}>", local.value()),
            };
            write!(
                w,
                "loan {} as {} with {} ; ",
                local,
                loan_as.text(db),
                with.text(db)
            )?;
            let mut new_locals = locals.to_vec();
            new_locals.insert(0, loan_as);
            new_locals.insert(0, with);
            write_expression(db, body, &new_locals, w)
        }
        ExpressionData::Take {
            local,
            proofs,
            body,
        } => {
            let local = match locals.get(local.value() as usize) {
                Some(local) => local.text(db).clone(),
                None => format!("<local {}>", local.value()),
            };
            write!(w, "take {} {{", local)?;
            for (name, proof) in proofs.iter() {
                let local = match locals.get(name.value() as usize) {
                    Some(local) => local.text(db).clone(),
                    None => format!("<local {}>", name.value()),
                };
                write!(w, "\n{local} -> ")?;
                write_expression(db, *proof, locals, w)?;
                write!(w, " ,")?;
            }
            write!(w, "\n}} ;\n")?;
            write_expression(db, body, locals, w)
        }
        ExpressionData::In { reference, target } => {
            write_expression(db, reference, locals, w)?;
            write!(w, " in ")?;
            write_expression(db, target, locals, w)
        }
    }
}

fn write_binder(
    db: &FeatherDatabase,
    binder: Binder,
    locals: &[Str],
    w: &mut impl Write,
) -> std::fmt::Result {
    write_binder_structure(db, binder.structure, locals, w)?;
    let mut new_locals = locals.to_vec();
    new_locals.insert(0, binder.structure.bound.name);
    write_expression(db, binder.body, &new_locals, w)
}

fn write_binder_structure(
    db: &FeatherDatabase,
    structure: BinderStructure,
    locals: &[Str],
    w: &mut impl Write,
) -> std::fmt::Result {
    match structure.argument_style {
        ArgumentStyle::Explicit => write!(w, "( ")?,
        ArgumentStyle::ImplicitEager => write!(w, "{{ ")?,
        ArgumentStyle::ImplicitWeak => write!(w, "{{{{ ")?,
    }
    write!(w, "{} : ", structure.bound.name.text(db))?;
    if structure.bound.usage == Usage::Erased {
        write!(w, "0 ")?;
    }
    write_expression(db, structure.bound.ty, locals, w)?;
    match structure.argument_style {
        ArgumentStyle::Explicit => write!(w, " )")?,
        ArgumentStyle::ImplicitEager => write!(w, " }}")?,
        ArgumentStyle::ImplicitWeak => write!(w, " }}}}")?,
    }
    match structure.invocation_style {
        InvocationStyle::Once => write!(w, " -> ")?,
        InvocationStyle::Many => write!(w, " => ")?,
    }
    Ok(())
}

impl kernel::Db for FeatherDatabase {
    fn format_expression(&self, expr: Expression) -> String {
        // The formatter only works on whole source files,
        // so we need to essentially embed this expression in a source file.
        const INITIAL: &str = "module print def f: Sort 0 = ";
        let mut input = INITIAL.to_owned();
        match write_expression(self, expr, &[], &mut input) {
            Ok(()) => match formatter::format_feather(&input) {
                Some(result) => result[INITIAL.len()..].trim().to_owned(),
                None => format!("<failed to format expression: {input}>"),
            },
            Err(_) => unreachable!("should not error while writing to a string"),
        }
    }
}

impl FeatherDatabase {
    /// Returns the database, along with a receiver for file update events.
    /// If running as a language server, this channel should be watched,
    /// and any updated paths should be processed by the database.
    /// If running as a standalone compiler, the channel may be ignored,
    /// although receiving file update events may still be desirable in certain cases.
    pub fn new(
        project_root: PathBuf,
    ) -> (Self, mpsc::Receiver<notify_debouncer_mini::DebouncedEvent>) {
        let (tx, rx) = mpsc::channel();
        let debouncer = notify_debouncer_mini::new_debouncer(
            Duration::from_secs(1),
            None,
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(events) => events.into_iter().for_each(|e| tx.send(e).unwrap()),
                Err(errors) => errors.iter().for_each(|e| panic!("{e:?}")),
            },
        )
        .unwrap();

        let this = Self {
            storage: Default::default(),
            project_root,
            files: Default::default(),
            watcher: Arc::new(Mutex::new(debouncer)),
        };

        (this, rx)
    }
}
