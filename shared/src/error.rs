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
    DuplicateArg {
        this: &'a str,
    },
    /// An argument conflicts with others.
    ConflictArgs {
        this: &'a str,
        those: &'a [&'a str],
    },
    /// An argument misses requirements.
    MissingRequirements {
        those: &'a [&'a str],
    },
    /// An argument is undefined.
    UnknownArg {
        this: &'a str,
    },
    InvalidInput,
}

/// A formatter that should meet most cases.
pub struct DefaultFormatter {
    namespace: Option<&'static str>,
}

/// Builder for [`DefaultFormatter`].
#[derive(Default)]
pub struct DefaultFormatterBuilder {
    namespace: Option<&'static str>,
}

impl DefaultFormatterBuilder {
    /// Defines the namespace for arguments and formats each argument as `namespace.argument`.
    pub fn namespace(self, namespace: &'static str) -> Self {
        Self {
            namespace: Some(namespace),
            ..self
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
}

impl ErrorFormatter for DefaultFormatter {
    fn fmt(&self, err: &Error) -> String {
        match err {
            &Error::DuplicateArg { this } => {
                format!("{} is duplicate", FmtArg(self.namespace, this))
            }
            &Error::ConflictArgs { this, those } => format!(
                "{} conflicts with {}",
                FmtArg(self.namespace, this),
                FmtArgs(self.namespace, those),
            ),
            &Error::MissingRequirements { those } => {
                format!(
                    "{} {} required",
                    if those.len() > 1 { "are" } else { "is" },
                    FmtArgs(self.namespace, those),
                )
            }
            &Error::UnknownArg { this } => {
                format!("{} is unknown", FmtArg(self.namespace, this))
            }
            &Error::InvalidInput => {
                format!("invalid input")
            }
        }
    }
}

struct FmtArg<'a>(Option<&'a str>, &'a str);
impl fmt::Display for FmtArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ns) = self.0 {
            write!(f, "`{}.{}`", ns, self.1)
        } else {
            write!(f, "`{}`", self.1)
        }
    }
}

struct FmtArgs<'a>(Option<&'a str>, &'a [&'a str]);
impl<'a> fmt::Display for FmtArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.1.iter();
        if let Some(arg) = iter.next() {
            write!(f, "{}", FmtArg(self.0, arg))?;
        }
        for arg in iter {
            write!(f, ", {}", FmtArg(self.0, arg))?;
        }
        Ok(())
    }
}
