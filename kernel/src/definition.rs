use crate::expr::{Expression, Usage};

use files::{Str, WithProvenance};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    pub name: WithProvenance<Str>,
    pub usage: Usage,
    pub ty: Expression,
    pub body: Expression,
}
