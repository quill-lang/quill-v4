use std::{fmt::Debug, path::PathBuf, sync::Arc};

use diagnostic::{miette, Dr};
use miette::Diagnostic;
use thiserror::Error;

#[salsa::jar(db = Db)]
pub struct Jar(Str, Path, InputFile, Source, source);

pub trait Db: std::fmt::Debug + salsa::DbWithJar<Jar> {
    /// Loads source code from a file.
    /// This is performed lazily when needed.
    fn input_file(&self, path: std::path::PathBuf) -> std::io::Result<InputFile>;
}

/// A span of code in a given source file.
/// Represented by a range of UTF-8 characters.
/// See also [`SourceSpan`].
///
/// The default span is `0..0`.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// The lower bound of the span (inclusive).
    pub start: usize,
    /// The upper bound of the span (exclusive).
    pub end: usize,
}

impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(value: std::ops::Range<usize>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<&std::ops::Range<usize>> for Span {
    fn from(value: &std::ops::Range<usize>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(value: Span) -> Self {
        value.start..value.end
    }
}

impl From<Span> for miette::SourceSpan {
    fn from(value: Span) -> Self {
        Self::new(value.start.into(), (value.end - value.start).into())
    }
}

pub trait Spanned {
    fn span(&self) -> Span;
}

/// An interned string type.
/// Can be safely copied and compared cheaply.
#[salsa::interned]
pub struct Str {
    #[return_ref]
    pub text: String,
}

/// Generates a sequence of distinct strings with a given prefix.
pub struct StrGenerator<'a> {
    db: &'a dyn Db,
    prefix: String,
    counter: u64,
}

impl<'a> StrGenerator<'a> {
    pub fn new(db: &'a dyn Db, prefix: impl ToString) -> Self {
        Self {
            db,
            prefix: prefix.to_string(),
            counter: 0,
        }
    }

    pub fn generate(&mut self) -> Str {
        let result = Str::new(
            self.db,
            if self.counter == 0 {
                self.prefix.clone()
            } else {
                format!("{}_{}", self.prefix, self.counter)
            },
        );
        self.counter += 1;
        result
    }
}

/// A fully qualified path.
/// Can be used, for example, as a qualified name for a definition or for a source file.
/// Can be safely copied and compared cheaply.
#[salsa::interned]
pub struct Path {
    #[return_ref]
    pub segments: Vec<Str>,
}

impl Path {
    pub fn display(self, db: &dyn Db) -> String {
        self.segments(db)
            .iter()
            .map(|s| s.text(db))
            .cloned()
            .collect::<Vec<_>>()
            .join("::")
    }

    /// Split the last element off a path and return the resulting components.
    /// If a path was `[a, b, c]`, this function returns `([a, b], c)`.
    /// Typically this is used for extracting the name of the source file and the item inside that module from a qualified name.
    ///
    /// # Panics
    ///
    /// If this path does not have any elements, this will panic.
    pub fn split_last(&self, db: &dyn Db) -> (Path, Str) {
        let (last_element, source_file) = self.segments(db).split_last().unwrap();
        (Path::new(db, Vec::from(source_file)), *last_element)
    }

    pub fn with(self, db: &dyn Db, segment: Str) -> Path {
        let mut segments = self.segments(db).clone();
        segments.push(segment);
        Path::new(db, segments)
    }

    pub fn append(self, db: &dyn Db, segments: impl IntoIterator<Item = Str>) -> Path {
        let mut original_segments = self.segments(db).clone();
        original_segments.extend(segments);
        Path::new(db, original_segments)
    }

    pub fn to_path_buf(&self, db: &dyn Db) -> PathBuf {
        self.segments(db).iter().map(|s| s.text(db)).collect()
    }

    pub fn to_string(&self, db: &dyn Db) -> String {
        self.segments(db)
            .iter()
            .map(|s| s.text(db).to_owned())
            .collect::<Vec<_>>()
            .join("::")
    }
}

/// Uniquely identifies a source file.
#[salsa::interned]
pub struct Source {
    /// The relative path from the project root to this source file.
    /// File extensions are *not* appended to this path.
    pub path: Path,
    /// The type of the file.
    /// This is used to deduce the file extension.
    pub ty: SourceType,
}

/// Used to deduce the file extension of a [`Source`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceType {
    /// A feather source file, encoded as UTF-8.
    Feather,
    /// A quill source file, encoded as UTF-8.
    Quill,
}

impl SourceType {
    pub fn extension(self) -> &'static str {
        match self {
            SourceType::Feather => "ftr",
            SourceType::Quill => "qll",
        }
    }
}

/// A span of code in a particular source file.
/// See also [`Span`].
/// Not to be confused with [`miette::SourceSpan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub source: Source,
    pub span: Span,
}

impl SourceSpan {
    pub fn new(source: Source, span: Span) -> Self {
        Self { source, span }
    }
}

/// The origin of some data, if known.
/// If no data is provided, we say that the provenance is "synthetic".
pub type Provenance = Option<SourceSpan>;

/// Attaches provenance data to a type.
///
/// Note that in certain cases, especially with expression types, we attach provenance information
/// alongside the data in a second structure, rather than bundling it in each object as we do here.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WithProvenance<T> {
    /// The origin of the value.
    pub provenance: Provenance,
    /// The actual value.
    pub contents: T,
}

impl<T> WithProvenance<T> {
    pub fn new(provenance: Provenance, contents: T) -> Self {
        Self {
            provenance,
            contents,
        }
    }
}

/// An input file.
#[salsa::input]
pub struct InputFile {
    pub path: PathBuf,
    pub contents: Arc<String>,
}

#[tracing::instrument(level = "debug")]
#[salsa::tracked]
pub fn source(db: &dyn Db, source: Source) -> Dr<Arc<String>, SourceError> {
    let path_buf = source
        .path(db)
        .to_path_buf(db)
        .with_extension(source.ty(db).extension());
    match db.input_file(path_buf) {
        Ok(value) => Dr::new(value.contents(db)),
        Err(err) => Dr::new_err(SourceError {
            src: source.path(db).to_path_buf(db),
            message: err.to_string(),
        }),
    }
}

#[derive(Error, Diagnostic, Debug, Clone, Eq, PartialEq)]
#[error("error reading {src}: {message}")]
pub struct SourceError {
    src: PathBuf,
    message: String,
}
