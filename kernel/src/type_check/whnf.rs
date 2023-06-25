//! Converts expressions to weak head normal form.
//!
//! Conversion rules: <https://coq.inria.fr/refman/language/core/conversion.html>

use crate::{expr::*, Db};

impl Expression {
    /// Reduces an expression to weak head normal form.
    pub fn to_weak_head_normal_form(mut self, db: &dyn Db) -> Self {
        // loop {
        self = whnf_core(db, self);
        // match self.unfold_definition(db) {
        //     Some(new) => self = new,
        //     None => break,
        // }
        // }
        self
    }
}

/// Tries to put an expression in weak head normal form, but does not perform delta reduction.
fn whnf_core(db: &dyn Db, e: Expression) -> Expression {
    match e.data(db) {
        _ => e,
    }
}
