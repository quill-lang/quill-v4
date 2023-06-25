//! Utility functions on [`Expression`] using [`Expression::find`] and [`Expression::replace`]

use std::{cell::RefCell, cmp::Ordering};

use crate::{
    de_bruijn::{DeBruijnIndex, DeBruijnOffset},
    expr::*,
    type_check::{definition_height, DefinitionHeight},
    Db,
};

impl Expression {
    /// Returns the first local constant or hole in the given expression.
    #[must_use]
    pub fn first_local_or_hole(self, db: &dyn Db) -> Option<Self> {
        self.find(db, &|inner, _offset| {
            matches!(
                inner.data(db),
                ExpressionData::LocalConstant(_) | ExpressionData::Hole(_)
            )
        })
    }

    /// Returns true if the given hole appears in `self`.
    #[must_use]
    pub fn hole_occurs(self, db: &dyn Db, hole: HoleId) -> bool {
        self.find(db, &|inner, _offset| {
            if let ExpressionData::Hole(var) = inner.data(db) {
                hole == var.id
            } else {
                false
            }
        })
        .is_some()
    }

    /// Returns true if the local variable given by `local` appears in `self`.
    #[must_use]
    pub fn local_is_bound(self, db: &dyn Db, local: DeBruijnIndex) -> bool {
        self.find(db, &|inner, offset| {
            if let ExpressionData::Local(bound) = inner.data(db) {
                bound == local + offset
            } else {
                false
            }
        })
        .is_some()
    }

    /// Traverses the expression tree and calls the given function on each expression.
    /// The tree is traversed depth first.
    pub fn for_each_expression(self, db: &dyn Db, func: impl FnMut(Self, DeBruijnOffset)) {
        let cell = RefCell::new(func);
        self.find(db, &|inner, offset| {
            cell.borrow_mut()(inner, offset);
            false
        });
    }

    /// Gets the maximum height of reducible definitions contained inside this expression.
    #[must_use]
    pub fn get_max_height(self, db: &dyn Db) -> DefinitionHeight {
        let mut height = 0;
        self.for_each_expression(db, |inner, _offset| {
            if let ExpressionData::Inst(path) = inner.data(db) {
                if let Some(inner_height) = definition_height(db, path) {
                    height = std::cmp::max(height, inner_height);
                }
            }
        });
        height
    }

    /// Instantiate the first bound variable with the given substitution.
    /// This will subtract one from all higher de Bruijn indices.
    /// TODO: n-ary instantiation operation.
    #[must_use]
    pub fn instantiate(self, db: &dyn Db, substitution: Self) -> Self {
        self.replace(db, &|e, offset| {
            match e.data(db) {
                ExpressionData::Local(index) => {
                    match index.cmp(&(DeBruijnIndex::zero() + offset)) {
                        Ordering::Less => {
                            // The variable is bound and has index lower than the offset, so we don't change it.
                            ReplaceResult::Skip
                        }
                        Ordering::Equal => {
                            // The variable is the smallest free de Bruijn index.
                            // It is exactly the one we need to substitute.
                            ReplaceResult::ReplaceWith(substitution.lift_free_vars(
                                db,
                                DeBruijnOffset::zero(),
                                offset,
                            ))
                        }
                        Ordering::Greater => {
                            // This de Bruijn index must be decremented, since we just
                            // instantiated a variable below it.
                            ReplaceResult::ReplaceWith(Self::new_local(db, index.pred()))
                        }
                    }
                }
                _ => ReplaceResult::Skip,
            }
        })
    }

    /// Increase the de Bruijn indices of free variables by a certain offset.
    /// Before the check, we increase the index of each expression by `bias`.
    #[must_use]
    pub fn lift_free_vars(self, db: &dyn Db, bias: DeBruijnOffset, shift: DeBruijnOffset) -> Self {
        self.replace(db, &|e, offset| {
            match e.data(db) {
                ExpressionData::Local(index) => {
                    if index >= DeBruijnIndex::zero() + offset + bias {
                        // The variable is free.
                        ReplaceResult::ReplaceWith(Self::new_local(db, index + shift))
                    } else {
                        ReplaceResult::Skip
                    }
                }
                _ => ReplaceResult::Skip,
            }
        })
    }

    /// Create a lambda or pi binder where the parameter is the given local constant.
    /// Invoke this with a closed expression.
    #[must_use]
    pub fn abstract_binder(self, db: &dyn Db, local: LocalConstant) -> Binder {
        let return_type = self.replace(db, &|e, offset| match e.data(db) {
            ExpressionData::LocalConstant(inner_local) => {
                if inner_local == local {
                    ReplaceResult::ReplaceWith(Self::new_local(db, DeBruijnIndex::zero() + offset))
                } else {
                    ReplaceResult::Skip
                }
            }
            _ => ReplaceResult::Skip,
        });

        Binder {
            structure: local.structure,
            body: return_type,
        }
    }

    /// Replaces every instance of the given hole inside this expression with a replacement.
    #[must_use]
    pub fn fill_hole(self, db: &dyn Db, id: HoleId, replacement: Self) -> Self {
        self.replace(db, &|e, offset| match e.data(db) {
            ExpressionData::Hole(hole) => {
                if hole.id == id {
                    ReplaceResult::ReplaceWith(replacement.lift_free_vars(
                        db,
                        DeBruijnOffset::zero(),
                        offset,
                    ))
                } else {
                    ReplaceResult::Skip
                }
            }
            _ => ReplaceResult::Skip,
        })
    }

    /// Replace the given local constant with this expression.
    #[must_use]
    pub fn replace_local(self, db: &dyn Db, local: &LocalConstant, replacement: Self) -> Self {
        self.replace(db, &|e, offset| {
            if let ExpressionData::LocalConstant(inner) = e.data(db) {
                if inner.id == local.id {
                    // We should replace this local variable.
                    ReplaceResult::ReplaceWith(replacement.lift_free_vars(
                        db,
                        DeBruijnOffset::zero(),
                        offset,
                    ))
                } else {
                    ReplaceResult::Skip
                }
            } else {
                ReplaceResult::Skip
            }
        })
    }
}
