//! Unfolds definitions.

use files::Path;

use crate::{
    expr::{Expression, ExpressionData},
    get_certified_definition, Db,
};

use super::definition::{DefinitionHeight, Reducibility};

/// Returns the height of the definition that this [`Path`] refers to.
/// If this instance could not be resolved, was not a definition, or was not reducible, return [`None`].
pub fn definition_height(db: &dyn Db, path: Path) -> Option<DefinitionHeight> {
    get_certified_definition(db, path).as_ref().and_then(|def| {
        if let Reducibility::Reducible { height } = def.reducibility() {
            Some(height)
        } else {
            None
        }
    })
}

impl Expression {
    /// Returns a number if the head of this expression is a definition that we can unfold.
    /// Intuitively, the number returned is higher for more complicated definitions.
    pub fn head_definition_height(self, db: &dyn Db) -> Option<DefinitionHeight> {
        match self.data(db) {
            ExpressionData::Inst(path) => definition_height(db, path),
            ExpressionData::Apply { left, .. } => left.head_definition_height(db),
            _ => None,
        }
    }

    /// If the head of this expression is a definition, unfold it.
    /// This is sometimes called delta-reduction.
    /// If the definition was marked [`Reducibility::Irreducible`], do nothing.
    ///
    /// If we couldn't unfold anything, return [`None`].
    /// This will always return a value if [`head_definition_height`] returned a [`Some`] value.
    pub fn unfold_definition(self, db: &dyn Db) -> Option<Self> {
        match self.data(db) {
            ExpressionData::Inst(path) => {
                get_certified_definition(db, path).as_ref().and_then(|def| {
                    match def.reducibility() {
                        Reducibility::Reducible { .. } => def.def().body,
                        Reducibility::Irreducible => None,
                    }
                })
            }
            ExpressionData::Apply { left, right } => left
                .unfold_definition(db)
                .map(|e| Expression::new_apply(db, e, right)),
            _ => None,
        }
    }
}
