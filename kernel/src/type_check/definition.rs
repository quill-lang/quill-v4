use crate::{definition::Definition, expr::Universe};

use files::Path;
use std::fmt::Display;

/// A definition that has been verified by the type checker.
/// No data inside a certified definition can be changed; this preserves the certification status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertifiedDefinition {
    def: Definition,
    /// The type of the type of the definition, stored as a universe level.
    universe: Universe,
    reducibility: Reducibility,
    /// Why this definition exists.
    origin: DefinitionOrigin,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DefinitionOrigin {
    /// This definition was written directly in Feather code.
    Feather,
    /// This definition is the type declaration for an inductive type.
    TypeDeclaration { inductive: Path },
    /// This definition is an intro rule for an inductive type.
    IntroRule { inductive: Path },
}

impl CertifiedDefinition {
    /// Certified definitions can only be created by the type checker in the kernel.
    pub(in crate::type_check) fn new(
        def: Definition,
        universe: Universe,
        reducibility: Reducibility,
        origin: DefinitionOrigin,
    ) -> Self {
        Self {
            def,
            universe,
            reducibility,
            origin,
        }
    }

    pub fn def(&self) -> &Definition {
        &self.def
    }

    pub fn universe(&self) -> Universe {
        self.universe
    }

    pub fn reducibility(&self) -> Reducibility {
        self.reducibility
    }

    pub fn origin(&self) -> DefinitionOrigin {
        self.origin
    }
}

/// Information used by the definitional equality checker to choose which definitions to unfold first.
/// In particular, if we are checking if `f x y z` is equal to `g a b c`, we look at the
/// reducibility information of `f` and `g`. If one has a heigher height than the other, we unfold
/// that one first, as it may reduce into an invocation of the other function. This essentially
/// allows us to unfold complicated expressions into easier ones, rather than having to unfold
/// all expressions into normal form, which would be very computationally intensive.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Reducibility {
    Reducible {
        height: DefinitionHeight,
    },
    /// Irreducible definitions are never unfolded.
    /// They do not have a definition height.
    /// Irreducible definitions include recursive functions that may not terminate.
    Irreducible,
}

impl Display for Reducibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reducibility::Reducible { height } => {
                write!(f, "reducible definition with height {height}")
            }
            Reducibility::Irreducible => write!(f, "irreducible definition"),
        }
    }
}

/// If this number is higher, the definition is 'more complex'.
/// We define the height of a [`Reducibility::Reducible`] definition to be one more than
/// the maximum height of any [`Reducibility::Reducible`] definitions it contains.
pub type DefinitionHeight = u64;
