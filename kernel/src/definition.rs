use crate::expr::{Expression, Usage};

use files::{Str, WithProvenance};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    pub name: WithProvenance<Str>,
    pub usage: Usage,
    pub ty: Expression,
    /// Empty if the body contained an error or was not given.
    pub body: Option<Expression>,
}
