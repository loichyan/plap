#![cfg_attr(docsrs, feature(doc_cfg))]

mod arg;
mod checker;
#[macro_use]
mod define_args;
mod error;
mod parser;
#[cfg(feature = "string")]
mod str;

pub use arg::{Arg, ArgAttrs, ArgKind};
pub use checker::{AnyArg, Checker};
pub use define_args::{ArgEnum, Args};
pub use error::{Error, Errors};
pub use parser::{Optional, Parser};

pub type Result<T, E = error::Error> = std::result::Result<T, E>;
pub type OptionalArg<T> = Arg<Optional<T>>;

/// **NOT PUBLIC APIS**
#[doc(hidden)]
pub mod private {
    pub use crate::*;

    pub mod arg {
        use super::*;
        use proc_macro2::{Ident, Span};

        pub type StructParseResult = crate::Result<Span>;
        pub type EnumParseResult<T> = crate::Result<(Ident, T)>;

        pub fn new_attrs() -> ArgAttrs {
            ArgAttrs::default()
        }

        pub fn parse_key(parser: &mut Parser) -> syn::Result<Ident> {
            // do not move the cursor unless we find an acknowledged argument
            parser.peek_key()
        }

        pub fn is_key(key: &Ident, expected: &str) -> bool {
            key == expected
        }

        pub fn parse_add_value<T>(
            parser: &mut Parser,
            attrs: &ArgAttrs,
            key: Ident,
            a: &mut Arg<T>,
        ) -> StructParseResult
        where
            T: syn::parse::Parse,
        {
            // now we can move the cursor
            let span = parser.consume_next()?.unwrap();
            a.add(key, parser.next_value(attrs)?);
            Ok(span)
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
            parser.consume_next()?.unwrap();
            let value = parser.next_value(attrs)?;
            Ok((key, variant(value)))
        }

        pub fn unknown_argument<T>(key: Ident) -> crate::Result<T> {
            Err(crate::Error::UnknownArgument(key))
        }
    }
}
