pub mod arg;
pub mod error;
pub mod group;
pub mod parser;
pub mod runtime;

pub struct Name(RawName);

const DUMMY_NAME: RawName = RawName::Borrowed("<UNDEFINED>");

type RawName = std::borrow::Cow<'static, str>;

impl From<&'static str> for Name {
    fn from(val: &'static str) -> Self {
        Self(RawName::Borrowed(val))
    }
}

impl From<String> for Name {
    fn from(val: String) -> Self {
        Self(RawName::Owned(val))
    }
}
