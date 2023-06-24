use files::{Path, Str};

use crate::{vec_map::VecMap, Db, DeBruijnIndex};

#[salsa::tracked]
pub struct Expression {
    pub data: ExpressionData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpressionData {
    Local(DeBruijnIndex),
    Apply {
        /// The function to invoke.
        left: Expression,
        /// The parameter to supply.
        right: Expression,
    },
    Lambda(Binder),
    Pi(Binder),
    Let {
        /// The name of the local variable being declared.
        name: Str,
        /// The expression to assign to the local variable.
        to_assign: Expression,
        /// The body of the expression, where `name` is given de Bruijn index 0.
        body: Expression,
    },
    Sort(Universe),
    Inst(Path),
    Intro {
        /// The path of the datatype or proposition type to introduce.
        path: Path,
        /// The (global, then index) parameters of the type.
        parameters: Vec<Expression>,
        /// The name of the variant to instantiate.
        variant: Str,
        /// The fields of the newly created object.
        /// This is a list of key-value pairs.
        fields: VecMap<Str, Expression>,
    },
    Match {
        /// The object we are matching against.
        subject: Expression,
        /// The return value of the match expression.
        /// This should be a 1-argument function that takes something of the type of `subject`
        /// and returns something in `Sort u` for a fixed u.
        /// If the subject is a `Prop`, the allowed values of `u` depend on the variants in the
        /// propositional type in question.
        return_ty: Expression,
        /// The list of cases for each field.
        /// Each entry `(k, v)` is an `n`-argument function, where `n` is the number of fields
        /// in variant `k`. The return type of each case is given by `return_ty`.
        cases: VecMap<Str, Expression>,
    },
    Fix {
        /// The type of the `fix` expression.
        binder: Binder,
        /// The name of the local variable that can be invoked to recursively calculate the body.
        rec_name: Str,
        /// The body of the `fix` expression.
        /// - local variable `0` is the `fix` body with name `rec_name`;
        /// - local variable `1` is the subject of the fixpoint recursion, named in `binder`.
        body: Expression,
    },
    /// A type of references.
    Ref(Expression),
    /// Dereference the inner expression.
    Dereference(Expression),
    Loan {
        /// The local variable to loan.
        local: DeBruijnIndex,
        /// The name of the variable that will be assigned a reference of `name`.
        loan_as: Str,
        /// The name of the variable that will store a proof that `name` and `*loan_as` are equal.
        with: Str,
        /// The body of the expression, in which
        /// - local variable `0` is `with`;
        /// - local variable `1` is `loan_as`.
        body: Expression,
    },
    Take {
        /// The local variable that we want to cancel the loan of.
        local: DeBruijnIndex,
        /// The proofs that the borrow is not stored in any newly created local variable.
        proofs: VecMap<DeBruijnIndex, Expression>,
        /// The body after we `take` the local variable back.
        body: Expression,
    },
    In {
        /// An expression of a reference type.
        reference: Expression,
        /// The target of the `in` expression.
        target: Expression,
    },
}

impl Expression {
    /// Creates a new `Local` expression.
    pub fn local(db: &dyn Db, index: DeBruijnIndex) -> Expression {
        Expression::new(db, ExpressionData::Local(index))
    }

    /// Creates a new `Lambda` expression.
    pub fn lambda(db: &dyn Db, binder: Binder) -> Expression {
        Expression::new(db, ExpressionData::Lambda(binder))
    }

    /// Creates a new `Pi` expression.
    pub fn pi(db: &dyn Db, binder: Binder) -> Expression {
        Expression::new(db, ExpressionData::Pi(binder))
    }

    /// Creates a new `Sort` expression.
    pub fn sort(db: &dyn Db, universe: Universe) -> Expression {
        Expression::new(db, ExpressionData::Sort(universe))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Usage {
    Erased,
    Present,
}

/// A bound variable in a lambda, pi, or let expression.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BoundVariable {
    /// The name of the local variable to bind.
    pub name: Str,
    /// The type of the value assigned to the bound variable.
    pub ty: Expression,
    /// The multiplicity for which the value is bound.
    pub usage: Usage,
}

/// How should the argument to this function be given?
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ArgumentStyle {
    /// The argument is to be given explicitly.
    Explicit,
    /// The argument is implicit, and is to be filled eagerly by the elaborator.
    ImplicitEager,
    /// The argument is implicit, and is to be filled by the elaborator only when another later parameter is given.
    ImplicitWeak,
}

/// How should the function be called?
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InvocationStyle {
    /// The function is to be called exactly once.
    Once,
    /// The function can be called arbitrarily many times, from behind a borrow.
    Many,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BinderStructure {
    /// The local variable to bind.
    pub bound: BoundVariable,
    /// How the parameter should be filled when calling the function.
    pub argument_style: ArgumentStyle,
    /// How the function should be called.
    pub invocation_style: InvocationStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Binder {
    pub structure: BinderStructure,
    pub body: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Universe(pub u32);
