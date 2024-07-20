use syn::parse::{Parse, ParseBuffer};
use syn::Result;

use crate::{ExprArg, FlagArg, NamedArg};

/// An extension trait for [`ParseBuffer`].
pub trait ParseStreamExt {
    /// Parses as a [`NamedArg`] and returns its value.
    fn parse_named_arg<T: Parse>(&self) -> Result<T>;

    /// Parses as a [`ExprArg`] and returns its value.
    fn parse_expr_arg<T: Parse>(&self) -> Result<T>;

    /// Parses as a [`FlagArg`] and returns `true`.
    fn parse_flag_arg(&self) -> Result<bool>;
}

impl ParseStreamExt for ParseBuffer<'_> {
    fn parse_named_arg<T: Parse>(&self) -> Result<T> {
        self.parse::<NamedArg<T>>().map(|t| t.value)
    }

    fn parse_expr_arg<T: Parse>(&self) -> Result<T> {
        self.parse::<ExprArg<T>>().map(|t| t.value)
    }

    fn parse_flag_arg(&self) -> Result<bool> {
        self.parse::<FlagArg>().map(|_| true)
    }
}
