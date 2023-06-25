#![feature(trait_upcasting)]

pub mod de_bruijn;
pub mod definition;
pub mod expr;
pub mod type_check;
pub mod vec_map;

use definition::Definition;
use diagnostic::DynDr;
use files::Path;
use type_check::definition::{CertifiedDefinition, DefinitionOrigin};

pub trait Db: files::Db + salsa::DbWithJar<Jar> {
    fn format_expression(&self, expr: expr::Expression) -> String;

    /// Given a fully qualified path of a definition in a either a feather or a quill file,
    /// return the parsed and elaborated definition.
    /// This definition will not have been type checked.
    fn get_definition_impl(&self, path: Path) -> DynDr<Definition>;
}

/// Given a fully qualified path of a definition in a either a feather or a quill file,
/// return the parsed and elaborated definition.
/// This definition will not have been type checked.
#[salsa::tracked(return_ref)]
pub fn get_definition(db: &dyn Db, path: Path) -> DynDr<Definition> {
    db.get_definition_impl(path)
}

/// Type checks the definition with the given name.
/// This function returns a [`CertifiedDefinition`], a definition that has been verified by the type checker.
///
/// See also [`type_check::certify_definition`].
///
/// # Usage
///
/// When type checking a definition, we may depend on previously certified definitions.
/// These should only be accessed using [`get_certified_definition`], so that we don't double any error messages emitted.
#[salsa::tracked(return_ref)]
pub fn certify_definition(db: &dyn Db, path: Path) -> DynDr<CertifiedDefinition> {
    get_definition(db, path).clone().bind(|def| {
        type_check::certify_definition(db, path, &def, DefinitionOrigin::Feather).to_dynamic()
    })
}

/// Type checks the definition with the given name, or retrieves it from the database if it was already type checked.
/// This function returns a [`CertifiedDefinition`], a definition that has been verified by the type checker.
/// This function will discard any diagnostic messages produced by type checking the definition.
#[salsa::tracked(return_ref)]
pub fn get_certified_definition(db: &dyn Db, path: Path) -> Option<CertifiedDefinition> {
    certify_definition(db, path).value().cloned()
}

#[salsa::jar(db = Db)]
pub struct Jar(
    expr::Expression,
    get_definition,
    certify_definition,
    get_certified_definition,
);
