use proc_macro2::{Ident, Span};
use syn::parse::{Parse, ParseStream};
use syn::{parenthesized, LitStr, Token};

use crate::arg::{ArgAttrs, ArgKind};

pub struct Parser<'a> {
    input: ParseStream<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: ParseStream<'a>) -> Self {
        Self { input }
    }

    pub fn input(&self) -> ParseStream<'a> {
        self.input
    }

    pub fn span(&self) -> Span {
        self.input.span()
    }

    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    pub fn is_eoa(&self) -> bool {
        self.input.peek(Token![,]) || self.input.is_empty()
    }

    pub fn next_key(&mut self) -> syn::Result<Ident> {
        self.input.parse::<Option<Ident>>().and_then(|i| match i {
            Some(i) => Ok(i),
            None => Err(self.input.error("expected an identifier")),
        })
    }

    pub fn peek_key(&mut self) -> syn::Result<Ident> {
        self.input
            .cursor()
            .ident()
            .ok_or_else(|| self.input.error("expected an identifier"))
            .map(|(i, _)| i)
    }

    pub fn next_value<T: Parse>(&mut self, attrs: &ArgAttrs) -> syn::Result<T> {
        self.next_value_with(attrs, T::parse)
    }

    pub fn next_value_with<T>(
        &mut self,
        attrs: &ArgAttrs,
        f: impl FnOnce(ParseStream) -> syn::Result<T>,
    ) -> syn::Result<T> {
        let input = self.input;
        let kind = attrs.get_kind();

        if self.is_eoa() {
            match kind {
                ArgKind::Expr | ArgKind::TokenTree => {
                    if attrs.get_optional() {
                        return parse_value_from_str("", f);
                    }
                }
                ArgKind::Flag => return parse_value_from_str("true", f),
                _ => {}
            }
        }

        match kind {
            ArgKind::Expr | ArgKind::Flag => {
                if input.parse::<Option<Token![=]>>()?.is_some() && !self.is_eoa() {
                    f(input)
                } else if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    f(&content)
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
            ArgKind::Help => parse_value_from_str("", f),
        }
    }

    pub fn next_eoa(&mut self) -> syn::Result<Option<Span>> {
        if let Some(c) = self.input.parse::<Option<Token![,]>>()? {
            Ok(Some(c.span))
        } else if self.is_empty() {
            Ok(None)
        } else {
            Err(self.input.error("expected a `,`"))
        }
    }

    /// Consumes the next token and returns its span. If it reaches
    /// [`EOF`](Self::is_eof), [`None`] is returned.
    pub fn consume_next(&mut self) -> syn::Result<Option<Span>> {
        self.input
            .parse::<Option<proc_macro2::TokenTree>>()
            .map(|t| t.map(|t| t.span()))
    }

    pub fn parse_all_with(
        &mut self,
        mut f: impl FnMut(&mut Self) -> syn::Result<Option<Span>>,
    ) -> syn::Result<()> {
        let mut errors = crate::errors::Errors::default();
        loop {
            if self.is_empty() {
                break;
            }

            match f(self) {
                Ok(Some(_)) => {
                    if errors.add_result(self.next_eoa()).is_some() {
                        continue;
                    }
                }
                Ok(None) => errors.add_at(self.span(), "unknown argument"),
                Err(e) => errors.add(e),
            }

            // eat all unexpected tokens
            loop {
                if self.is_eoa() {
                    self.consume_next()?;
                    break;
                }
                self.consume_next()?;
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

#[derive(Debug)]
pub struct Optional<T>(pub Option<T>);

impl<T: Parse> Parse for Optional<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(Self(None))
        } else {
            input.parse().map(Some).map(Self)
        }
    }
}
