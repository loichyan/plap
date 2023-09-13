use std::marker::PhantomData;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    token, Ident, LitBool, Result, Token,
};

/// Parses an argument and returns its value.
pub trait ArgParser {
    type Value: Sized;

    fn parse(self, input: ParseStream) -> Result<Self::Value>;
}

macro_rules! generic_parser {
    ($(#[$attr:meta])* $vis:vis struct $name:ident<$g:ident>;) => {
        $(#[$attr])* $vis struct $name<$g>(PhantomData<$g>);
        impl<$g> Default for $name<$g> {
            fn default() -> Self {
                Self::new()
            }
        }
        impl<$g> $name<$g> {
            pub const fn new() -> Self {
                Self(PhantomData)
            }
        }
    };
}

generic_parser!(
    /// Parses input as a positional argument, i.e. tokens are parsed to the
    /// value directly.
    pub struct PositionalArgParser<T>;
);

impl<T> ArgParser for PositionalArgParser<T>
where
    T: Parse,
{
    type Value = T;

    fn parse(self, input: ParseStream) -> Result<Self::Value> {
        input.parse()
    }
}

generic_parser!(
    /// Parses input as a named AST argument, i.e. tokens appear inside
    /// parentheses are parsed directly, while ones to the right of equal token
    /// are parsed as quoted to support arbitrary input. For example,
    /// `name = "Value"` and `name(Value)` are parsed to the same result.
    pub struct NamedAstArgParser<T>;
);

impl<T> ArgParser for NamedAstArgParser<T>
where
    T: Parse,
{
    type Value = T;

    fn parse(self, input: ParseStream) -> Result<Self::Value> {
        let _ = input.parse::<Ident>()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(token::Paren) {
            let content;
            let _ = parenthesized!(content in input);
            content.parse()
        } else if lookahead.peek(Token![=]) {
            let _ = input.parse::<Token![=]>()?;
            let s = input.parse::<syn::LitStr>()?;
            s.parse()
        } else {
            Err(lookahead.error())
        }
    }
}

generic_parser!(
    /// Parses input as a named expression argument, i.e. regardless of whether
    /// tokens appear inside parentheses or to the right of equal token, they
    /// are parsed directly. For example, `name = value` and `name(value)` are
    /// parsed to the same result.
    pub struct NamedExprArgParser<T>;
);

impl<T> ArgParser for NamedExprArgParser<T>
where
    T: Parse,
{
    type Value = T;

    fn parse(self, input: ParseStream) -> Result<Self::Value> {
        let _ = input.parse::<Ident>()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(token::Paren) {
            let content;
            let _ = parenthesized!(content in input);
            content.parse()
        } else if lookahead.peek(Token![=]) {
            let _ = input.parse::<Token![=]>()?;
            input.parse()
        } else {
            Err(lookahead.error())
        }
    }
}

/// Parses input as a flag argument, i.e. tokens are treated as a literal
/// boolean expression while empty input produces a default `true` value.
pub struct FlagArgParser;

impl ArgParser for FlagArgParser {
    type Value = bool;

    fn parse(self, input: ParseStream) -> Result<Self::Value> {
        let _ = input.parse::<Ident>()?;
        Ok(if input.peek(token::Paren) {
            let content;
            let _ = parenthesized!(content in input);
            content.parse::<LitBool>()?.value
        } else if input.peek(Token![=]) {
            let _ = input.parse::<Token![=]>()?;
            input.parse::<LitBool>()?.value
        } else {
            true
        })
    }
}
