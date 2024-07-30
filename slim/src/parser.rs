use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;
use syn::{parenthesized, LitStr, Token};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ArgKind {
    Expr,
    Flag,
    TokenTree,
    Help,
}

impl Default for ArgKind {
    fn default() -> Self {
        ArgKind::TokenTree
    }
}

pub struct Parser<'a> {
    input: ParseStream<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: ParseStream<'a>) -> Self {
        Self { input }
    }

    pub fn is_eof(&self) -> bool {
        self.input.is_empty()
    }

    pub fn is_eoa(&self) -> bool {
        self.input.peek(Token![,]) || self.is_eof()
    }

    pub fn next_key(&mut self) -> syn::Result<Ident> {
        self.input.parse::<Option<Ident>>().and_then(|i| match i {
            Some(i) => Ok(i),
            None => Err(syn::Error::new(
                consume(self.input)?,
                "expected an identifier",
            )),
        })
    }

    pub fn next_value<T>(&mut self, kind: ArgKind) -> syn::Result<T>
    where
        T: syn::parse::Parse,
    {
        self.next_value_with(kind, T::parse)
    }

    pub fn next_value_with<T>(
        &mut self,
        kind: ArgKind,
        f: impl FnOnce(ParseStream) -> syn::Result<T>,
    ) -> syn::Result<T> {
        let input = self.input;

        let r = match kind {
            ArgKind::Expr | ArgKind::Flag => {
                if input.parse::<Option<Token![=]>>()?.is_some() {
                    f(input)
                } else if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    f(&content)
                } else if kind == ArgKind::Flag && self.is_eoa() {
                    parse_value_from_str("true", f)
                        .map_err(|e| panic!("a flag parser must support `true`: {}", e))
                } else {
                    Err(input.error("expected `= <value>` or `(<value>)`"))
                }
            }
            ArgKind::TokenTree => {
                if input.parse::<Option<Token![=]>>()?.is_some() {
                    let content = input.parse::<syn::LitStr>()?;
                    parse_value_from_literal(content, f)
                } else if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    f(&content)
                } else {
                    Err(input.error("expected `= \"<value>\"` or `(<value>)`"))
                }
            }
            ArgKind::Help => parse_value_from_str("", f)
                .map_err(|e| panic!("a help parser must support ``: {}", e)),
        }?;

        if !self.is_eof() && input.parse::<Option<Token![,]>>()?.is_none() {
            Err(syn::Error::new(consume(input)?, "expected a `,`"))
        } else {
            Ok(r)
        }
    }

    /// Consumes all tokens up to the EOA.
    ///
    /// If no error is returned, it stops at the beginning of the next argument
    /// and returns the last span of all consumed tokens. This is typically used
    /// to eat unexpected tokens of the current argument if an error occurs
    /// during parsing.
    pub fn consume_next(&mut self) -> syn::Result<Option<Span>> {
        let input = self.input;
        let mut last = None;
        while input.parse::<Option<Token![,]>>()?.is_none() && !input.is_empty() {
            last = Some(consume(input)?);
        }
        Ok(last)
    }

    pub fn parse_all_with(
        &mut self,
        mut f: impl FnMut(&mut Self) -> syn::Result<Result<(), Ident>>,
    ) -> syn::Result<()> {
        let mut errors = crate::errors::Errors::default();
        loop {
            if self.is_eof() {
                break;
            }

            match f(self) {
                Ok(Ok(_)) => continue,
                Ok(Err(unknown)) => errors.add_at(unknown.span(), "unknown argument"),
                Err(e) => errors.add(e),
            }

            // eat all unexpected tokens
            if let Some(span) = self.consume_next()? {
                errors.add_at(span, "unexpected tokens");
            }
        }
        errors.fail()
    }

    pub fn parse_all<A>(&mut self, args: &mut A) -> syn::Result<()>
    where
        A: crate::define_args::Args,
    {
        self.parse_all_with(|parser| A::parse_next(args, parser))
    }
}

fn consume(input: ParseStream) -> syn::Result<Span> {
    input.parse::<proc_macro2::TokenTree>().map(|t| t.span())
}

fn parse_value_from_str<T>(
    input: &str,
    f: impl FnOnce(ParseStream) -> syn::Result<T>,
) -> syn::Result<T> {
    let input = LitStr::new(input, Span::call_site());
    parse_value_from_literal(input, f)
}

fn parse_value_from_literal<T>(
    input: LitStr,
    f: impl FnOnce(ParseStream) -> syn::Result<T>,
) -> syn::Result<T> {
    input.parse_with(|input: ParseStream| f(input))
}
