//! Converts expressions to weak head normal form.
//!
//! Conversion rules: <https://coq.inria.fr/refman/language/core/conversion.html>

use crate::{expr::*, Db};

impl Expression {
    /// Reduces an expression to weak head normal form.
    #[must_use]
    pub fn weak_head_normal_form(mut self, db: &dyn Db) -> Self {
        loop {
            self = self.whnf_core(db);
            match self.unfold_definition(db) {
                Some(new) => self = new,
                None => break,
            }
        }
        self
    }

    /// Tries to put an expression in weak head normal form, but does not perform delta reduction.
    fn whnf_core(self, db: &dyn Db) -> Expression {
        match self.data(db) {
            ExpressionData::Apply { left, right } => {
                // Reduce the function to weak head normal form first.
                let left = left.whnf_core(db);
                match left.data(db) {
                    ExpressionData::Lambda(binder) => {
                        // If the function is a lambda, we can apply a beta-reduction to expand the lambda.
                        binder.body.instantiate(db, right).whnf_core(db)
                    }
                    ExpressionData::Fix { body, .. } => {
                        // If the function is a fixpoint expression, we can apply a fix-reduction to expand it.
                        body.instantiate(db, left)
                            .instantiate(db, right)
                            .whnf_core(db)
                    }
                    _ => Expression::new_apply(db, left, right),
                }
            }
            ExpressionData::Let {
                to_assign, body, ..
            } => {
                // We substitute the value into the body of the let expression, then continue to evaluate the expression.
                // This is called zeta-reduction.
                body.instantiate(db, to_assign).whnf_core(db)
            }
            ExpressionData::Match {
                subject,
                return_ty,
                cases,
            } => {
                // Reduce the major premise to weak head normal form first.
                let subject = subject.weak_head_normal_form(db);
                if let ExpressionData::Intro {
                    variant, fields, ..
                } = subject.data(db)
                {
                    // We can unfold this match expression.
                    // Since the match expression is type correct, the unwrap is ok.
                    // This is called match-reduction.
                    let (_, result) = cases
                        .iter()
                        .find(|(name, _)| *name == variant)
                        .copied()
                        .unwrap();

                    fields
                        .iter()
                        .fold(result, |result, (_, field)| {
                            Expression::new_apply(db, result, *field)
                        })
                        .whnf_core(db)
                } else {
                    Expression::new_match(db, subject, return_ty, cases)
                }
            }
            ExpressionData::Fix {
                binder,
                rec_name,
                body,
            } => todo!(),
            ExpressionData::Ref(_) => todo!(),
            ExpressionData::Deref(_) => todo!(),
            ExpressionData::Loan {
                local,
                loan_as,
                with,
                body,
            } => todo!(),
            ExpressionData::Take {
                local,
                proofs,
                body,
            } => todo!(),
            ExpressionData::In { reference, target } => todo!(),
            ExpressionData::LocalConstant(_) => todo!(),
            ExpressionData::Hole(_) => todo!(),
            _ => self,
        }
    }
}
