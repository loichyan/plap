mod arg;
mod ast;
mod error;
mod ext;
mod parser;
mod runtime;

pub use arg::*;
pub use ast::*;
pub use error::*;
pub use ext::*;
pub use parser::*;

use proc_macro2::Span;
use syn::{parse::ParseStream, Result};

type Name = &'static str;

/// The user-defined container of a set of arguments.
pub trait Args: Sized {
    type Parser: Parser<Output = Self>;

    /// Gets the associated [`Parser`].
    fn parser(node: Span) -> Self::Parser;

    /// Parses an input stream.
    fn parse(input: ParseStream) -> Result<Self> {
        let mut parser = Self::parser(Span::call_site());
        parser.parse(input)?;
        parser.finish()
    }
}
