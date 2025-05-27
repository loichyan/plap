#![cfg_attr(docsrs, feature(doc_cfg))]

mod arg;
mod checker;
#[macro_use]
mod define_args;
mod error;
mod parser;
#[cfg(feature = "string")]
mod str;

pub use arg::{Arg, ArgDesc, ArgEnum, ArgKind, Args};
pub use checker::{AnyArg, Checker};
pub use error::{Error, Errors};
pub use parser::{Optional, Parser};

pub type Result<T, E = error::Error> = std::result::Result<T, E>;
pub type OptionalArg<T> = Arg<Optional<T>>;

/// NON-PUBLIC API
#[doc(hidden)]
pub mod r#priv {
    pub use crate::*;
    use proc_macro2::{Ident, Span};

    pub type StructParseResult = crate::Result<Span>;
    pub type EnumParseResult<T> = crate::Result<(Ident, T)>;

    pub fn parse_args<T>(
        parser: &mut Parser,
        attrs: &ArgDesc,
        key: Ident,
        args: &mut Arg<T>,
    ) -> StructParseResult
    where
        T: syn::parse::Parse,
    {
        // Consume the peeked key
        let span = parser.consume_next()?.unwrap();
        args.add(key, parser.next_value(attrs)?);
        Ok(span)
    }

    pub fn parse_args_enum<T, U>(
        parser: &mut Parser,
        attrs: &ArgDesc,
        key: Ident,
        variant: fn(T) -> U,
    ) -> EnumParseResult<U>
    where
        T: syn::parse::Parse,
    {
        // Consume the peeked key
        parser.consume_next()?.unwrap();
        let value = parser.next_value(attrs)?;
        Ok((key, variant(value)))
    }

    pub fn unknown_argument<T>(key: Ident) -> crate::Result<T> {
        Err(crate::Error::UnknownArgument(key))
    }
}
