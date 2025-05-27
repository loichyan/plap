#![cfg_attr(docsrs, feature(doc_cfg))]

mod arg;
mod checker;
#[macro_use]
mod define_args;
mod errors;
mod parser;
#[cfg(feature = "string")]
mod str;

pub use arg::{Arg, ArgAttrs, ArgKind};
pub use checker::{AnyArg, Checker};
pub use define_args::{ArgEnum, Args};
pub use errors::Errors;
pub use parser::{Optional, Parser};

pub type OptionalArg<T> = Arg<Optional<T>>;

/// **NOT PUBLIC APIS**
#[doc(hidden)]
pub mod private {
    pub use crate::*;

    pub mod arg {
        use super::*;
        use proc_macro2::{Ident, Span};

        type ParseResult<T> = syn::Result<Option<T>>;
        pub type StructParseResult = ParseResult<Span>;
        pub type EnumParseResult<T> = ParseResult<(Ident, T)>;

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
            Ok(Some(span))
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
            Ok(Some((key, variant(value))))
        }

        pub fn unknown_argument<T>(_key: Ident) -> ParseResult<T> {
            Ok(None)
        }
    }
}
