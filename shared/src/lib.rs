#[macro_use]
mod util;

mod ast;
mod error;
mod ext;
mod parser;
mod runtime;

pub mod arg;

#[doc(inline)]
pub use {
    arg::{Arg, ArgAction, ArgGroup},
    ast::*,
    error::*,
    ext::*,
    parser::*,
};

use proc_macro2::Span;
use syn::{parse::ParseStream, Result};

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
