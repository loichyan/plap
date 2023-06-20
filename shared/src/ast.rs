use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    token, Ident, LitStr, Result, Token,
};

/// An argument followed by a delimited value.
///
/// Supported syntaxes:
///
/// - Parenthesized: `#[name(value)]`
/// - Bracketed: `#[name[value]]`
/// - Braced: `#[name{value}]`
/// - Quoted: `#[name="value"]`
pub struct NamedArg<T> {
    pub name: Ident,
    pub delimiter: ArgDelimiter,
    pub value: T,
}

impl<T> Parse for NamedArg<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let (delimiter, value) = parse_delimited(input, |input| input.parse::<LitStr>()?.parse())?;
        Ok(Self {
            name,
            delimiter,
            value,
        })
    }
}

/// A delimited argument whose value can appear right of equal token without quotes.
///
/// Supported syntaxes:
///
/// - Parenthesized: `#[name(value)]`
/// - Bracketed: `#[name[value]]`
/// - Braced: `#[name{value}]`
/// - Quoted: `#[name=value]`
pub struct ExprArg<T> {
    pub name: Ident,
    pub delimiter: ArgDelimiter,
    pub value: T,
}

impl<T> Parse for ExprArg<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let (delimiter, value) = parse_delimited(input, T::parse)?;
        Ok(Self {
            name,
            delimiter,
            value,
        })
    }
}

/// A grouping token of an argument.
pub enum ArgDelimiter {
    Paren(token::Paren),
    Bracket(token::Bracket),
    Brace(token::Brace),
    Eq(Token![=]),
}

/// A flag argument without a value.
///
/// Supported syntax: `#[name]`
pub struct FlagArg {
    pub name: Ident,
}

impl Parse for FlagArg {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
        })
    }
}

pub(crate) fn parse_delimited<T>(
    input: ParseStream,
    parse_eq: fn(ParseStream) -> Result<T>,
) -> Result<(ArgDelimiter, T)>
where
    T: Parse,
{
    let lookahead = input.lookahead1();
    let (delimiter, content, value);
    if lookahead.peek(token::Paren) {
        delimiter = ArgDelimiter::Paren(parenthesized!(content in input));
        value = content.parse()?;
    } else if lookahead.peek(token::Bracket) {
        delimiter = ArgDelimiter::Bracket(bracketed!(content in input));
        value = content.parse()?;
    } else if lookahead.peek(token::Brace) {
        delimiter = ArgDelimiter::Brace(braced!(content in input));
        value = content.parse()?;
    } else if lookahead.peek(Token![=]) {
        delimiter = input.parse().map(ArgDelimiter::Eq)?;
        value = parse_eq(input)?;
    } else {
        return Err(lookahead.error());
    }
    Ok((delimiter, value))
}
