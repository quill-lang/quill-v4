//! Find-and-replace operations on expressions.

use crate::{de_bruijn::DeBruijnOffset, expr::*, Db};

pub enum ReplaceResult {
    /// The expression should not be replaced.
    Skip,
    /// The expression should be replaced with the given value.e.
    ReplaceWith(Expression),
}

impl Expression {
    /// Traverses the expression tree and finds expressions matching the provided replacement function.
    /// If any matched, the replacement function generates the value to replace the found value with.
    /// The provided [`DeBruijnOffset`] gives the amount of binders the [`Expression`] argument is currently under.
    #[must_use]
    pub fn replace(
        self,
        db: &dyn Db,
        replace_fn: &impl Fn(Self, DeBruijnOffset) -> ReplaceResult,
    ) -> Self {
        self.replace_offset(db, replace_fn, DeBruijnOffset::zero())
    }

    /// Like [`Expression::replace`] but keeps track of sub-expression de Bruijn index offsets.
    #[must_use]
    fn replace_offset(
        self,
        db: &dyn Db,
        replace_fn: &impl Fn(Self, DeBruijnOffset) -> ReplaceResult,
        offset: DeBruijnOffset,
    ) -> Self {
        // Invoke the replacement function.
        match replace_fn(self, offset) {
            ReplaceResult::Skip => {
                // Traverse the sub-expressions of `self`.
                match self.data(db) {
                    ExpressionData::Local(_) => self,
                    ExpressionData::Apply { left, right } => Expression::new_apply(
                        db,
                        left.replace_offset(db, replace_fn, offset),
                        right.replace_offset(db, replace_fn, offset),
                    ),
                    ExpressionData::Lambda(mut binder) => {
                        binder.structure.bound.ty = binder
                            .structure
                            .bound
                            .ty
                            .replace_offset(db, replace_fn, offset);
                        binder.body = binder.body.replace_offset(db, replace_fn, offset.succ());
                        Expression::new_lambda(db, binder)
                    }
                    ExpressionData::Pi(mut binder) => {
                        binder.structure.bound.ty = binder
                            .structure
                            .bound
                            .ty
                            .replace_offset(db, replace_fn, offset);
                        binder.body = binder.body.replace_offset(db, replace_fn, offset.succ());
                        Expression::new_pi(db, binder)
                    }
                    ExpressionData::Let {
                        name,
                        to_assign,
                        body,
                    } => Expression::new_let(
                        db,
                        name,
                        to_assign.replace_offset(db, replace_fn, offset),
                        body.replace_offset(db, replace_fn, offset.succ()),
                    ),
                    ExpressionData::Sort(_) => self,
                    ExpressionData::Inst(_) => self,
                    ExpressionData::Intro {
                        path,
                        parameters,
                        variant,
                        fields,
                    } => Expression::new_intro(
                        db,
                        path,
                        parameters
                            .iter()
                            .map(|param| param.replace_offset(db, replace_fn, offset))
                            .collect(),
                        variant,
                        fields
                            .into_iter()
                            .map(|(name, value)| {
                                (name, value.replace_offset(db, replace_fn, offset))
                            })
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                    ExpressionData::Match {
                        subject,
                        return_ty,
                        cases,
                    } => Expression::new_match(
                        db,
                        subject.replace_offset(db, replace_fn, offset),
                        return_ty.replace_offset(db, replace_fn, offset),
                        cases
                            .into_iter()
                            .map(|(name, value)| {
                                (name, value.replace_offset(db, replace_fn, offset))
                            })
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                    ExpressionData::Fix {
                        mut binder,
                        rec_name,
                        body,
                    } => {
                        binder.structure.bound.ty = binder
                            .structure
                            .bound
                            .ty
                            .replace_offset(db, replace_fn, offset);
                        binder.body = binder.body.replace_offset(db, replace_fn, offset.succ());
                        Expression::new_fix(
                            db,
                            binder,
                            rec_name,
                            body.replace_offset(db, replace_fn, offset.succ().succ()),
                        )
                    }
                    ExpressionData::Ref(ty) => {
                        Expression::new_ref(db, ty.replace_offset(db, replace_fn, offset))
                    }
                    ExpressionData::Deref(value) => {
                        Expression::new_deref(db, value.replace_offset(db, replace_fn, offset))
                    }
                    ExpressionData::Loan {
                        local,
                        loan_as,
                        with,
                        body,
                    } => Expression::new_loan(
                        db,
                        local,
                        loan_as,
                        with,
                        body.replace_offset(db, replace_fn, offset.succ().succ()),
                    ),
                    ExpressionData::Take {
                        local,
                        proofs,
                        body,
                    } => Expression::new_take(
                        db,
                        local,
                        proofs
                            .into_iter()
                            .map(|(name, proof)| {
                                (name, proof.replace_offset(db, replace_fn, offset))
                            })
                            .collect::<Vec<_>>()
                            .into(),
                        body.replace_offset(db, replace_fn, offset),
                    ),
                    ExpressionData::In { reference, target } => Expression::new_in(
                        db,
                        reference.replace_offset(db, replace_fn, offset),
                        target.replace_offset(db, replace_fn, offset),
                    ),
                    ExpressionData::LocalConstant(mut constant) => {
                        constant.structure.bound.ty = constant
                            .structure
                            .bound
                            .ty
                            .replace_offset(db, replace_fn, offset);
                        Expression::new_local_constant(db, constant)
                    }
                    ExpressionData::Hole(mut hole) => {
                        hole.ty = hole.ty.replace_offset(db, replace_fn, offset);
                        Expression::new_hole(db, hole)
                    }
                }
            }
            ReplaceResult::ReplaceWith(replaced) => {
                // We replace `self` with the given value.
                // We don't try to traverse the sub-expressions of this returned value.
                replaced
            }
        }
    }

    /// Traverses the expression tree and finds expressions matching the provided predicate.
    /// If any return `true`, the first such expression is returned.
    /// The tree is traversed depth first.
    pub fn find(
        self,
        db: &dyn Db,
        predicate: &impl Fn(Self, DeBruijnOffset) -> bool,
    ) -> Option<Self> {
        self.find_offset(db, predicate, DeBruijnOffset::zero())
    }

    /// Like [`Expression::find`] but keeps track of sub-expression de Bruijn index offsets.
    fn find_offset(
        self,
        db: &dyn Db,
        predicate: &impl Fn(Self, DeBruijnOffset) -> bool,
        offset: DeBruijnOffset,
    ) -> Option<Self> {
        if predicate(self, offset) {
            Some(self)
        } else {
            match self.data(db) {
                ExpressionData::Local(_) => None,
                ExpressionData::Apply { left, right } => left
                    .find_offset(db, predicate, offset)
                    .or_else(|| right.find_offset(db, predicate, offset)),
                ExpressionData::Lambda(binder) | ExpressionData::Pi(binder) => binder
                    .structure
                    .bound
                    .ty
                    .find_offset(db, predicate, offset)
                    .or_else(|| binder.body.find_offset(db, predicate, offset.succ())),
                ExpressionData::Let {
                    to_assign, body, ..
                } => to_assign
                    .find_offset(db, predicate, offset)
                    .or_else(|| body.find_offset(db, predicate, offset.succ())),
                ExpressionData::Sort(_) => None,
                ExpressionData::Inst(_) => None,
                ExpressionData::Intro {
                    parameters, fields, ..
                } => parameters
                    .iter()
                    .find_map(|param| param.find_offset(db, predicate, offset))
                    .or_else(|| {
                        fields
                            .iter()
                            .find_map(|(_name, value)| value.find_offset(db, predicate, offset))
                    }),
                ExpressionData::Match {
                    subject,
                    return_ty,
                    cases,
                } => subject
                    .find_offset(db, predicate, offset)
                    .or_else(|| return_ty.find_offset(db, predicate, offset))
                    .or_else(|| {
                        cases
                            .iter()
                            .find_map(|(_name, value)| value.find_offset(db, predicate, offset))
                    }),
                ExpressionData::Fix { binder, body, .. } => binder
                    .structure
                    .bound
                    .ty
                    .find_offset(db, predicate, offset)
                    .or_else(|| binder.body.find_offset(db, predicate, offset.succ()))
                    .or_else(|| body.find_offset(db, predicate, offset.succ().succ())),
                ExpressionData::Ref(ty) => ty.find_offset(db, predicate, offset),
                ExpressionData::Deref(value) => value.find_offset(db, predicate, offset),
                ExpressionData::Loan { body, .. } => {
                    body.find_offset(db, predicate, offset.succ().succ())
                }
                ExpressionData::Take { proofs, body, .. } => proofs
                    .iter()
                    .find_map(|(_name, proof)| proof.find_offset(db, predicate, offset))
                    .or_else(|| body.find_offset(db, predicate, offset)),
                ExpressionData::In { reference, target } => reference
                    .find_offset(db, predicate, offset)
                    .or_else(|| target.find_offset(db, predicate, offset)),
                ExpressionData::LocalConstant(constant) => constant
                    .structure
                    .bound
                    .ty
                    .find_offset(db, predicate, offset),
                ExpressionData::Hole(hole) => hole.ty.find_offset(db, predicate, offset),
            }
        }
    }
}
