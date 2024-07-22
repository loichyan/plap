#[macro_use]
mod util;
mod ast;
mod error;
mod ext;
pub mod id;
mod parser;
pub mod parser2;
mod runtime;
pub mod schema;

pub mod arg;

use proc_macro2::Span;
use syn::parse::ParseStream;
use syn::Result;
#[doc(inline)]
pub use {
    arg::{Arg, ArgAction, ArgGroup},
    ast::*,
    error::*,
    ext::*,
    parser::*,
};

type Name = &'static str;

/// The user-defined container of a set of arguments.
pub trait Args: Sized {
    type Parser: Parser<Output = Self>;

    /// Gets the associated [`Parser`] with the default configuration.
    fn parser(node: Span) -> Self::Parser {
        Self::parser_from(ParserContext::new(node))
    }

    /// Gets the associated [`Parser`] with the pre-configured context.
    fn parser_from(context: ParserContext) -> Self::Parser {
        <Self::Parser as Parser>::from_context(context)
    }

    /// Parses an input stream.
    fn parse(input: ParseStream) -> Result<Self> {
        let mut parser = Self::parser(Span::call_site());
        parser.parse(input)?;
        parser.finish()
    }
}
