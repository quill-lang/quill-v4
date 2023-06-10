// Re-export `miette`.
pub use miette;

use std::{
    error::Error,
    fmt::{Debug, Display},
};

use miette::{Diagnostic, MietteDiagnostic};

/// An uninhabited type.
/// It is not possible to construct `x: Void` in safe Rust.
/// This is a zero-sized type.
///
/// We provide a variety of trait implementations for `Void`.
/// Of course, none of these can ever be called, but allows us to satisfy certain type bounds.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Void {}

impl Debug for Void {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl Display for Void {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl Error for Void {}

impl Diagnostic for Void {}

/// A diagnostic result that tracks both fatal and non-fatal diagnostics.
/// Non-fatal diagnostics can represent warnings, or simply advice given to the user.
///
/// This structure has two states, `ok` and `err`.
/// In the `ok` state, there is a value of type `T`, and a list of non-fatal diagnostics of type `N`.
/// In the `err` state, there is a fatal error of type `E`, and a list of non-fatal diagnostics of type `N`.
///
/// The default non-fatal error type is [`Void`], which can never be diagnostics.
/// This means that, by default, we do not track or allocate for non-fatal diagnostics.
///
/// We implement various monadic operations to compose diagnostic results.
/// When we hit the first fatal error, we will no longer track any subsequent diagnostics.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Dr<T, E = MietteDiagnostic, N = Void> {
    value: Result<T, E>,
    non_fatal: Vec<N>,
}

impl<T, E, N> Debug for Dr<T, E, N> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Ok(value) => write!(f, "{:?}", value),
            Err(_) => write!(f, "<fatal error>"),
        }
    }
}

impl<T, E, N> Dr<T, E, N> {
    /// Creates a new diagnostic result containing the given value, and no messages.
    /// The returned message is in the `ok` state.
    pub fn new(value: T) -> Self {
        Dr {
            value: Ok(value),
            non_fatal: Vec::new(),
        }
    }

    /// Creates a new diagnostic result containing the given fatal error.
    /// The returned message is in the `err` state.
    pub fn new_err(error: E) -> Self {
        Dr {
            value: Err(error),
            non_fatal: Vec::new(),
        }
    }

    /// Returns true if this diagnostic result is in the `ok` state.
    /// In this case, there is a value of type `T` contained in this struct.
    pub fn is_ok(&self) -> bool {
        self.value.is_ok()
    }

    /// Returns true if this diagnostic result is in the `ok` state.
    /// In this case, there is a fatal error of type `E` contained in this struct.
    pub fn is_err(&self) -> bool {
        self.value.is_err()
    }

    /// Applies the given operation to the contained value, if it exists.
    /// If this diagnostic result is in the `err` state, no action is performed.
    pub fn map<U>(self, op: impl FnOnce(T) -> U) -> Dr<U, E, N> {
        Dr {
            value: self.value.map(op),
            non_fatal: self.non_fatal,
        }
    }

    /// Applies the given operation to the contained error, if it exists.
    /// If this diagnostic result is in the `ok` state, no action is performed.
    pub fn map_err<F>(self, op: impl FnOnce(E) -> F) -> Dr<T, F, N> {
        Dr {
            value: self.value.map_err(op),
            non_fatal: self.non_fatal,
        }
    }

    /// Produces a new diagnostic result by adding the given non-fatal diagnostic.
    /// If this diagnostic result is in the `err` state, no action is performed.
    pub fn with(mut self, diag: N) -> Self {
        if self.is_ok() {
            self.non_fatal.push(diag);
        }
        self
    }

    /// Composes two diagnostic results, where the second may depend on the value inside the first.
    /// If `self` is in the `err` state, no action is performed, and an `err`-state [`Dr`] is returned.
    /// Otherwise, the non-fatal error messages of both diagnostic results are combined to produce the output.
    pub fn bind<U>(mut self, f: impl FnOnce(T) -> Dr<U, E, N>) -> Dr<U, E, N> {
        match self.value {
            Ok(value) => {
                let mut result = f(value);
                self.non_fatal.extend(result.non_fatal);
                result.non_fatal = self.non_fatal;
                result
            }
            Err(err) => Dr {
                value: Err(err),
                non_fatal: self.non_fatal,
            },
        }
    }

    /// Prints all of the diagnostic messages contained in this diagnostic result.
    /// Then, return the contained value, if present.
    pub fn print_diagnostics(self) -> Option<T>
    where
        E: Diagnostic + Send + Sync + 'static,
        N: Diagnostic + Send + Sync + 'static,
    {
        for diag in self.non_fatal {
            println!("{:?}", miette::Report::new(diag));
        }

        match self.value {
            Ok(value) => Some(value),
            Err(err) => {
                println!("{:?}", miette::Report::new(err));
                None
            }
        }
    }
}
