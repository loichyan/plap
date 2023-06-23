use crate::Name;
use std::fmt;

/// Customize validation [`Error`]s.
pub trait ErrorFormatter {
    /// Formats a validation [`Error`].
    fn fmt(&self, err: &Error) -> String;
}

impl<T> ErrorFormatter for &T
where
    T: ?Sized + ErrorFormatter,
{
    fn fmt(&self, err: &Error) -> String {
        T::fmt(self, err)
    }
}

impl<T> ErrorFormatter for Box<T>
where
    T: ?Sized + ErrorFormatter,
{
    fn fmt(&self, err: &Error) -> String {
        T::fmt(self, err)
    }
}

/// A validation error.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error<'a> {
    /// An argument is supplied multiple times.
    DuplicateArg { this: &'a str },
    /// Any of required arguments is not supplied.
    MissingRequired {
        this: Option<&'a str>,
        required: &'a [&'a str],
    },
    /// An argument conflicts with another.
    ArgConflict { this: &'a str, conflict: &'a str },
    /// An argument is unexpected in some node.
    UnexpectedArg { this: &'a str },
    /// An argument is undefined.
    UnknownArg { this: &'a str },
    /// Unexpected input tokens.
    InvalidInput,
}

/// A formatter that should meet most cases.
pub struct DefaultFormatter {
    namespace: Option<Name>,
}

/// Builder for [`DefaultFormatter`].
#[derive(Default)]
pub struct DefaultFormatterBuilder {
    namespace: Option<Name>,
}

impl DefaultFormatterBuilder {
    /// Defines the namespace for arguments and formats each argument as `namespace.argument`.
    pub fn namespace(self, namespace: Name) -> Self {
        Self {
            namespace: Some(namespace),
        }
    }

    /// Consumes the builder and constructs [`DefaultFormatter`].
    pub fn build(self) -> DefaultFormatter {
        let Self { namespace } = self;
        DefaultFormatter { namespace }
    }
}

impl Default for DefaultFormatter {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl DefaultFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures this formatter.
    pub fn builder() -> DefaultFormatterBuilder {
        DefaultFormatterBuilder::default()
    }

    /// Returns the current [`namespace`].
    ///
    /// [`namespace`]: DefaultFormatterBuilder::namespace
    pub fn namespace(&self) -> Option<&str> {
        self.namespace
    }

    fn fmt_arg<'a>(&'a self, name: &'a str) -> FmtArg {
        FmtArg(self, name)
    }

    fn fmt_args<'a>(&'a self, names: &'a [&'a str]) -> FmtArgs {
        FmtArgs(self, names)
    }
}

impl ErrorFormatter for DefaultFormatter {
    fn fmt(&self, err: &Error) -> String {
        match err {
            Error::DuplicateArg { this } => {
                format!("{} is duplicate", self.fmt_arg(this))
            }
            Error::MissingRequired {
                this: Some(this),
                required,
            } => {
                format!(
                    "{} requires {}",
                    self.fmt_arg(this),
                    self.fmt_args(required)
                )
            }
            Error::MissingRequired {
                this: None,
                required,
            } => {
                format!("requires {}", self.fmt_args(required))
            }
            Error::ArgConflict { this, conflict } => {
                format!(
                    "{} conflicts with {}",
                    self.fmt_arg(this),
                    self.fmt_arg(conflict),
                )
            }
            Error::UnexpectedArg { this } => {
                format!("{} is not allowed", self.fmt_arg(this))
            }
            Error::UnknownArg { this } => {
                format!("{} is unknown", self.fmt_arg(this))
            }
            Error::InvalidInput => "invalid input".to_owned(),
        }
    }
}

struct FmtArg<'a>(&'a DefaultFormatter, &'a str);
impl fmt::Display for FmtArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ns) = self.0.namespace {
            write!(f, "`{}.{}`", ns, self.1)
        } else {
            write!(f, "`{}`", self.1)
        }
    }
}

struct FmtArgs<'a>(&'a DefaultFormatter, &'a [&'a str]);
impl<'a> fmt::Display for FmtArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let FmtArgs(fm, grp) = self;
        match (grp.get(0), grp.get(1)) {
            (Some(n1), None) => {
                write!(f, "{}", fm.fmt_arg(n1))
            }
            (Some(n1), Some(n2)) => {
                write!(f, "{} or {}", fm.fmt_arg(n1), fm.fmt_arg(n2))
            }
            _ => {
                let mut iter = grp.iter();
                if let Some(arg) = iter.next() {
                    write!(f, "one of {}", fm.fmt_arg(arg))?;
                }
                for arg in iter {
                    write!(f, ", {}", fm.fmt_arg(arg))?;
                }
                Ok(())
            }
        }
    }
}
