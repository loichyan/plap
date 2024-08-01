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
            None => Err(self.input.error("expected an identifier")),
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
            Err(input.error("expected a `,`"))
        } else {
            Ok(r)
        }
    }

    /// Consumes the next token and returns its span. If it reaches
    /// [`EOA`](Self::is_eoa), [`None`] is returned.
    ///
    /// This is typically used to eat unexpected tokens of the current argument
    /// if an error occurs during parsing.
    pub fn consume_next(&mut self) -> syn::Result<Option<Span>> {
        if self.is_eoa() {
            Ok(None)
        } else {
            self.input
                .parse::<proc_macro2::TokenTree>()
                .map(|t| t.span())
                .map(Some)
        }
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
                Err(e) => {
                    errors.add(e);
                    // In most cases, the returned error points to the input's
                    // current span, so we need to skip the current token to
                    // prevent an "unexpected tokens" error from appearing in
                    // the same place.
                    if self.consume_next()?.is_none() {
                        continue;
                    }
                }
            }

            // eat all unexpected tokens
            while let Some(span) = self.consume_next()? {
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
