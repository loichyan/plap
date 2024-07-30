use std::{fmt, ops};

pub(crate) enum Str {
    Static(&'static str),
    Owned(Box<str>),
}

impl fmt::Debug for Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl Str {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Owned(s) => s,
        }
    }
}

impl From<&'static str> for Str {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}

impl From<String> for Str {
    fn from(s: String) -> Self {
        Self::Owned(s.into())
    }
}

impl ops::Deref for Str {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
