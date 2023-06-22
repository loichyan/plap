mod ast;
mod error;
mod ext;
mod parser;
mod runtime;

pub mod arg;

#[doc(inline)]
pub use {
    arg::{Arg, ArgAction},
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

    /// Gets the associated [`Parser`].
    fn parser(node: Span) -> Self::Parser {
        <Self::Parser as Parser>::with_node(node)
    }

    /// Parses an input stream.
    fn parse(input: ParseStream) -> Result<Self> {
        let mut parser = Self::parser(Span::call_site());
        parser.parse(input)?;
        parser.finish()
    }
}
