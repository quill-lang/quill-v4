use files::Str;

use crate::{Db, DeBruijnIndex};

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
