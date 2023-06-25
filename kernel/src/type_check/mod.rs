//! Performs type checking and evaluation of expressions.

use diagnostic::Dr;
use files::Path;

use crate::{definition::Definition, Db};

use self::definition::{CertifiedDefinition, DefinitionOrigin};

pub mod defeq;
pub mod definition;
pub mod whnf;

/// Type checks the definition with the given name.
/// This function returns a [`CertifiedDefinition`], a definition that has been verified by the type checker.
///
/// # Usage
///
/// Instead of calling this method directly, which takes a [`Definition`] as well as its [`Path`],
/// in most instances you should call [`Db::certify_definition`] or [`Db::get_certified_definition`].
/// These functions are able to parse and certify both feather and quill definitions.
pub fn certify_definition(
    db: &dyn Db,
    path: Path,
    def: &Definition,
    origin: DefinitionOrigin,
) -> Dr<CertifiedDefinition> {
    todo!()
}
