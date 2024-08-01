#[macro_use]
mod define_args;

mod arg;
mod checker;
mod errors;
mod parser;
#[cfg(feature = "string")]
mod str;

pub use arg::Arg;
pub use checker::{AnyArg, Checker};
pub use define_args::{ArgAttrs, ArgEnum, Args};
pub use errors::Errors;
pub use parser::{ArgKind, Parser};

/// **NOT PUBLIC APIS**
#[doc(hidden)]
pub mod private {
    use proc_macro2::Ident;
    pub use syn;

    pub use crate::*;

    pub mod arg {
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
