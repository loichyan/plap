#![cfg_attr(docsrs, feature(doc_cfg))]

mod arg;
#[macro_use]
mod define_args;
#[cfg(feature = "checking")]
mod checker;
mod errors;
mod parser;
#[cfg(feature = "string")]
mod str;

pub use arg::Arg;
#[cfg(feature = "checking")]
pub use checker::{AnyArg, Checker};
pub use define_args::{ArgAttrs, ArgEnum, Args};
pub use errors::Errors;
pub use parser::{ArgKind, Parser};

/// **NOT PUBLIC APIS**
#[doc(hidden)]
pub mod private {
    pub use crate::*;

    pub mod arg {
        use proc_macro2::Ident;

        use super::*;

        pub type ParseResult<T> = syn::Result<Result<T, Ident>>;
        pub type StructParseResult = ParseResult<()>;
        pub type EnumParseResult<T> = ParseResult<(Ident, T)>;

        pub fn new_attrs() -> ArgAttrs {
            ArgAttrs::default()
        }

        pub fn parse_key(parser: &mut Parser) -> syn::Result<Ident> {
            parser.next_key()
        }

        pub fn is_key(key: &Ident, expected: &str) -> bool {
            key == expected
        }

        pub fn parse_add_value<T>(
            parser: &mut Parser,
            attrs: &ArgAttrs,
            key: Ident,
            a: &mut Arg<T>,
        ) -> ParseResult<()>
        where
            T: syn::parse::Parse,
        {
            a.add(key, parser.next_value(attrs.get_kind())?);
            Ok(Ok(()))
        }

        pub fn parse_value_into<T, U>(
            parser: &mut Parser,
            attrs: &ArgAttrs,
            key: Ident,
            variant: fn(T) -> U,
        ) -> EnumParseResult<U>
        where
            T: syn::parse::Parse,
        {
            let value = parser.next_value(attrs.get_kind())?;
            Ok(Ok((key, variant(value))))
        }

        pub fn unknown_argument<T>(key: Ident) -> ParseResult<T> {
            Ok(Err(key))
        }
    }
}

/// **NOT PUBLIC APIS**
#[cfg(feature = "checking")]
#[doc(hidden)]
#[macro_export]
macro_rules! private {
    (@cfg(feature = "checking") $($tt:tt)*) => { $($tt)* };
}

/// **NOT PUBLIC APIS**
#[cfg(not(feature = "checking"))]
#[doc(hidden)]
#[macro_export]
macro_rules! private {
    (@cfg(feature = "checking") $($tt:tt)*) => {};
}
