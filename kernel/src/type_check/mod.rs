//! Performs type checking and evaluation of expressions.

use diagnostic::Dr;
use files::Path;

use crate::{definition::Definition, Db};

mod defeq;
mod definition;
mod unfold;
mod whnf;

pub use defeq::*;
pub use definition::*;
pub use unfold::*;
pub use whnf::*;

/// Type checks the definition with the given name.
/// This function returns a [`CertifiedDefinition`], a definition that has been verified by the type checker.
///
/// # Usage
///
/// Instead of calling this method directly, which takes a [`Definition`] as well as its [`Path`],
/// in most instances you should call [`crate::certify_definition`] or [`crate::get_certified_definition`].
/// These functions are able to parse and certify both feather and quill definitions.
pub fn certify_definition(
    _db: &dyn Db,
    _path: Path,
    _def: &Definition,
    _origin: DefinitionOrigin,
) -> Dr<CertifiedDefinition> {
    todo!()
}
