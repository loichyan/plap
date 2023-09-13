use crate::RawName;
use std::fmt;

#[non_exhaustive]
pub enum Error<'a> {
    /// Any of `args` must be supplied.
    MissingArgument {
        args: &'a [&'a str],
    },
    /// Conflicts with any of given arguments.
    ConflictingArgument {
        args: &'a [&'a str],
    },
    DuplicateValue,
}

pub trait ErrorFormatter {
    fn fmt(&self, f: &mut fmt::Formatter, err: &Error) -> fmt::Result;
}

pub(crate) enum DelegatedFormatter {
    Default(DefaultFormatter),
    Other(Box<dyn ErrorFormatter>),
}

impl ErrorFormatter for DelegatedFormatter {
    fn fmt(&self, f: &mut fmt::Formatter, err: &Error) -> fmt::Result {
        match self {
            Self::Default(t) => t.fmt(f, err),
            Self::Other(t) => t.fmt(f, err),
        }
    }
}

pub(crate) struct DefaultFormatter {
    pub namespace: Option<RawName>,
}

impl ErrorFormatter for DefaultFormatter {
    fn fmt(&self, f: &mut fmt::Formatter, err: &Error) -> fmt::Result {
        let ns = self.namespace.as_deref();
        match err {
            Error::MissingArgument { args } => {
                write!(f, "requires {}", FmtGrp(ns, args))
            }
            Error::ConflictingArgument { args: group } => {
                write!(f, "conflicts with {}", FmtGrp(ns, group))
            }
            Error::DuplicateValue => {
                write!(f, "duplicate value")
            }
        }
    }
}

struct FmtArg<'a>(Option<&'a str>, &'a str);

impl fmt::Display for FmtArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ns, name) = self;
        if let Some(ns) = ns {
            write!(f, "`{}.{}`", ns, name)
        } else {
            write!(f, "`{}`", name)
        }
    }
}

struct FmtGrp<'a>(Option<&'a str>, &'a [&'a str]);

impl fmt::Display for FmtGrp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ns, args) = self;
        match args.len() {
            // len == 0
            0 => Ok(()),
            // len == 1
            1 => write!(f, "{}", FmtArg(ns, args[0])),
            // len == 2
            2 => {
                write!(f, "{} or {}", FmtArg(ns, args[0]), FmtArg(ns, args[1]))
            }
            // len >= 3
            _ => {
                write!(f, "one of {}", FmtArg(ns, args[0]))?;
                for arg in &args[1..] {
                    write!(f, ", {}", FmtArg(ns, arg))?;
                }
                Ok(())
            }
        }
    }
}
