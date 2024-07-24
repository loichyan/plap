// Some implementation details are borrowed from: https://docs.rs/clap_builder/4.5.9/src/clap_builder/util/id.rs.html

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(inner::Inner);

impl Id {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl From<&'_ Id> for Id {
    fn from(id: &'_ Id) -> Self {
        Self(id.0.clone())
    }
}

impl std::borrow::Borrow<str> for Id {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

#[cfg(not(feature = "string"))]
mod inner {
    use super::*;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    #[repr(transparent)]
    pub(crate) struct Inner(&'static str);

    impl AsRef<str> for Id {
        fn as_ref(&self) -> &str {
            (self.0).0
        }
    }

    impl From<&'static str> for Id {
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

    impl AsRef<str> for Id {
        fn as_ref(&self) -> &str {
            match self.0 {
                Inner::Static(s) => s,
                Inner::Owned(ref s) => s,
            }
        }
    }

    impl From<&'static str> for Id {
        fn from(s: &'static str) -> Self {
            Self(Inner::Static(s))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "string")))]
    impl From<String> for Id {
        fn from(s: String) -> Self {
            Self(Inner::Owned(s.into()))
        }
    }

    #[cfg_attr(docsrs, doc(cfg(feature = "string")))]
    impl From<&'_ String> for Id {
        fn from(s: &'_ String) -> Self {
            Self(Inner::Owned(s.clone().into()))
        }
    }
}
