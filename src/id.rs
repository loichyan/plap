// Some implementation details are borrowed from: https://docs.rs/clap_builder/4.5.9/src/clap_builder/util/id.rs.html

use std::fmt;
use std::ops::Deref;

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(Str);

impl Id {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&'_ Id> for Id {
    fn from(id: &'_ Id) -> Self {
        Self(id.0.clone())
    }
}

impl From<&'static str> for Id {
    fn from(id: &'static str) -> Self {
        Self(Str::from(id))
    }
}

#[cfg(feature = "string")]
#[cfg_attr(docsrs, doc(cfg(feature = "string")))]
impl From<String> for Id {
    fn from(id: String) -> Self {
        Self(Str::from(id))
    }
}

#[cfg(feature = "string")]
#[cfg_attr(docsrs, doc(cfg(feature = "string")))]
impl From<&'_ String> for Id {
    fn from(id: &'_ String) -> Self {
        Self(Str::from(id))
    }
}

impl std::borrow::Borrow<str> for Id {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Str(inner::Inner);

impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl std::borrow::Borrow<str> for Str {
    fn borrow(&self) -> &str {
        self.deref()
    }
}

impl fmt::Debug for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl fmt::Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

#[cfg(not(feature = "string"))]
mod inner {
    use super::*;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub(crate) struct Inner(&'static str);

    impl AsRef<str> for Str {
        fn as_ref(&self) -> &str {
            (self.0).0
        }
    }

    impl From<&'static str> for Str {
        fn from(s: &'static str) -> Self {
            Self(Inner(s))
        }
    }
}

#[cfg(feature = "string")]
mod inner {
    use super::*;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub(crate) enum Inner {
        Static(&'static str),
        Owned(Box<str>),
    }

    impl AsRef<str> for Str {
        fn as_ref(&self) -> &str {
            match self.0 {
                Inner::Static(s) => s,
                Inner::Owned(ref s) => s,
            }
        }
    }

    impl From<&'static str> for Str {
        fn from(s: &'static str) -> Self {
            Self(Inner::Static(s))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "string")))]
    impl From<String> for Str {
        fn from(s: String) -> Self {
            Self(Inner::Owned(s.into()))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "string")))]
    impl From<&'_ String> for Str {
        fn from(s: &'_ String) -> Self {
            Self(Inner::Owned(s.clone().into()))
        }
    }
}
