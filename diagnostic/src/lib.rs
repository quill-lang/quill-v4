// Re-export `miette`.
pub use miette;

use std::{
    error::Error,
    fmt::{Debug, Display},
};

use miette::{Diagnostic, Report};

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
/// The fatal error type should either be [`Report`] or a [`Diagnostic`].
///
/// The default non-fatal error type is [`Void`], which can never be diagnostics.
/// This means that, by default, we do not track or allocate for non-fatal diagnostics.
///
/// We implement various monadic operations to compose diagnostic results.
/// When we hit the first fatal error, we will no longer track any subsequent diagnostics.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Dr<T, E = Report, N = Void> {
    value: Result<T, E>,
    non_fatal: Vec<N>,
}

impl<T, E, N> Debug for Dr<T, E, N>
where
    T: Debug,
{
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

    /// Retrieves the value inside this diagnostic result if in the `ok` state.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref().ok()
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

    /// Applies the given operation to the contained error, if it exists.
    /// If this diagnostic result is in the `ok` state, no action is performed.
    pub fn map_errs<O>(self, op: impl FnMut(N) -> O) -> Dr<T, E, O> {
        Dr {
            value: self.value,
            non_fatal: self.non_fatal.into_iter().map(op).collect(),
        }
    }

    /// Converts the error types into generic [`Report`]s.
    pub fn to_reports(self) -> Dr<T, Report, Report>
    where
        E: Diagnostic + Send + Sync + 'static,
        N: Diagnostic + Send + Sync + 'static,
    {
        Dr {
            value: match self.value {
                Ok(value) => Ok(value),
                Err(err) => Err(Report::new(err)),
            },
            non_fatal: self.non_fatal.into_iter().map(Report::new).collect(),
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

    /// Combines a list of diagnostic results into a single result by binding them all together.
    pub fn sequence(results: impl IntoIterator<Item = Dr<T, E, N>>) -> Dr<Vec<T>, E, N> {
        results.into_iter().fold(Dr::new(Vec::new()), |acc, i| {
            acc.bind(|mut list| {
                i.bind(|i| {
                    list.push(i);
                    Dr::new(list)
                })
            })
        })
    }
}

impl<T, E> Dr<T, E, E> {
    /// Creates a new diagnostic report with the given vector of errors.
    /// This must be nonempty.
    /// The last entry in this list is used as the fatal error, all others are marked as non-fatal.
    /// This choice makes the rendered order of the errors correct.
    pub fn new_err_many(mut errors: Vec<E>) -> Self {
        assert!(!errors.is_empty());
        Self {
            value: Err(errors.pop().unwrap()),
            non_fatal: errors,
        }
    }

    /// Converts a failed diagnostic into a successful diagnostic by wrapping
    /// the contained value in an `Option`.
    pub fn unfail(mut self) -> Dr<Option<T>, E, E> {
        let value = match self.value {
            Ok(value) => Some(value),
            Err(err) => {
                self.non_fatal.push(err);
                None
            }
        };
        Dr {
            value: Ok(value),
            non_fatal: self.non_fatal,
        }
    }

    /// Combines a list of diagnostic results into a single result by binding them all together.
    /// Any failed diagnostics will be excluded from the output, but their error messages will remain.
    /// Therefore, this function will never fail - it might just produce an empty list as its output.
    pub fn sequence_unfail(results: impl IntoIterator<Item = Dr<T, E, E>>) -> Dr<Vec<T>, E, E> {
        results.into_iter().fold(Dr::new(Vec::new()), |acc, i| {
            acc.bind(|mut list| {
                i.unfail().bind(|i| {
                    if let Some(i) = i {
                        list.push(i);
                    }
                    Dr::new(list)
                })
            })
        })
    }
}

impl<T> Dr<T, Report, Report> {
    /// Prints all of the diagnostic messages contained in this diagnostic result.
    /// Then, return the contained value, if present.
    pub fn print_reports(self) -> Option<T> {
        for diag in self.non_fatal {
            println!("{:?}", diag);
        }

        match self.value {
            Ok(value) => Some(value),
            Err(err) => {
                println!("{:?}", err);
                None
            }
        }
    }
}
