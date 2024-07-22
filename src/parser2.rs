use std::any::Any;
use std::collections::BTreeMap;

use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;
use syn::{parenthesized, Token};

use crate::id::Id;
use crate::schema::*;

#[derive(Debug)]
pub struct Arg<T> {
    i: Idx,
    spans: Vec<Span>,
    values: Vec<T>,
}

impl<T> Arg<T> {
    pub fn schema() -> ArgSchema {
        ArgSchema::default()
    }

    pub(crate) fn new(i: Idx) -> Self {
        Self {
            i,
            spans: <_>::default(),
            values: <_>::default(),
        }
    }

    pub fn add_value(&mut self, span: Span, value: T) {
        self.spans.push(span);
        self.values.push(value);
    }
}

#[derive(Debug)]
pub struct ArgGroup {
    i: Idx,
}

impl ArgGroup {
    pub fn schema() -> ArgGroupSchema {
        ArgGroupSchema::default()
    }

    pub(crate) fn new(i: Idx) -> Self {
        Self { i }
    }
}

pub struct Parser<'a> {
    s: &'a Schema,
    args: BTreeMap<Idx, &'a mut dyn AnyArg>,
    errors: Option<syn::Error>,
}

impl<'a> Parser<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            s: schema,
            args: <_>::default(),
            errors: <_>::default(),
        }
    }

    pub fn add_arg<T>(&mut self, arg: &'a mut Arg<T>) -> &mut Self
    where
        T: 'static + syn::parse::Parse,
    {
        if is_debug!() {
            self.s.ensure_arg_registered(arg.i);
        }
        self.args.insert(arg.i, arg);
        self
    }

    pub fn add_group(&mut self, group: &'a ArgGroup) -> &mut Self {
        if is_debug!() {
            self.s.ensure_group_registered(group.i);
        }
        self
    }

    pub fn get_arg<T: 'static>(&self, id: impl Into<Id>) -> Option<&Arg<T>> {
        self._get_arg(id.into())
    }

    fn _get_arg<T: 'static>(&self, id: Id) -> Option<&Arg<T>> {
        self.s
            .get_idx(&id)
            .and_then(|id| self.args.get(&id))
            .map(|arg| {
                arg.as_any()
                    .downcast_ref()
                    .unwrap_or_else(|| panic!("argument type mismatched"))
            })
    }

    pub fn get_arg_mut<T: 'static>(&mut self, id: impl Into<Id>) -> Option<&mut Arg<T>> {
        self._get_arg_mut(id.into())
    }

    fn _get_arg_mut<T: 'static>(&mut self, id: Id) -> Option<&mut Arg<T>> {
        self.s
            .get_idx(&id)
            .and_then(|id| self.args.get_mut(&id))
            .map(|arg| {
                arg.as_any_mut()
                    .downcast_mut()
                    .unwrap_or_else(|| panic!("argument type mismatched"))
            })
    }

    pub fn parse(&mut self, tokens: ParseStream) -> syn::Result<()> {
        loop {
            if tokens.is_empty() {
                break;
            }

            if let Err(e) = self.parse_next(tokens) {
                if let Some(ref mut err) = self.errors {
                    err.combine(e);
                } else {
                    self.errors = Some(e);
                }
            }

            // consume all tokens till the next comma
            while tokens.parse::<Option<Token![,]>>()?.is_none() && !tokens.is_empty() {
                tokens.parse::<proc_macro2::TokenTree>()?;
            }
        }
        Ok(())
    }

    fn parse_next(&mut self, tokens: ParseStream) -> syn::Result<()> {
        let ident = tokens.parse::<Ident>()?;
        let span = ident.span();
        let name = ident.to_string();
        let i = self
            .s
            .get_idx(&name)
            .ok_or_else(|| syn_error!(span, "unknown argument"))?;

        let inf = match &self
            .s
            .get_info(i)
            .unwrap_or_else(|| unreachable!("unknown index"))
            .kind
        {
            InfoKind::Arg(i) => i,
            _ => panic!("`{}` is not registered as an argument", name),
        };
        let arg = self
            .args
            .get_mut(&i)
            .unwrap_or_else(|| panic!("`{}` is not added to parser", name));

        match inf.typ {
            ArgType::Expr | ArgType::Flag => {
                if tokens.parse::<Option<Token![=]>>()?.is_some() {
                    arg.parse_value(span, tokens)?;
                } else if tokens.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in tokens);
                    arg.parse_value(span, &content)?;
                } else if inf.typ == ArgType::Flag && (tokens.peek(Token![,]) || tokens.is_empty())
                {
                    arg.parse_value_from_str(span, "true")?;
                } else {
                    return Err(syn_error!(
                        span,
                        "expected a value of `= <expr>` or `(<expr>)`"
                    ));
                }
            }
            ArgType::TokenTree => {
                if tokens.parse::<Option<Token![=]>>()?.is_some() {
                    let content = tokens.parse::<syn::LitStr>()?.value();
                    arg.parse_value_from_str(span, &content)?;
                } else if tokens.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in tokens);
                    arg.parse_value(span, &content)?;
                } else {
                    return Err(syn_error!(
                        span,
                        "expected a value of `= \"<tt*>\"` or `(<tt*>)`"
                    ));
                }
            }
            ArgType::Help => {
                // TODO: show more usage
                arg.parse_value_from_str(span, "")?;
                return Err(syn_error!(span, &inf.help));
            }
        }

        Ok(())
    }

    pub fn finish(self) -> syn::Result<()> {
        // TODO: validation
        match self.errors {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

/// A type earsed and object safe [`Arg<T>`].
trait AnyArg {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn spans(&self) -> &[Span];

    fn parse_value(&mut self, span: Span, tokens: ParseStream) -> syn::Result<()>;

    fn parse_value_from_str(&mut self, span: Span, tokens: &str) -> syn::Result<()>;
}

impl<T: 'static + syn::parse::Parse> AnyArg for Arg<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn spans(&self) -> &[Span] {
        &self.spans
    }

    fn parse_value(&mut self, span: Span, tokens: ParseStream) -> syn::Result<()> {
        self.add_value(span, tokens.parse()?);
        Ok(())
    }

    fn parse_value_from_str(&mut self, span: Span, tokens: &str) -> syn::Result<()> {
        self.add_value(span, syn::parse_str(tokens)?);
        Ok(())
    }
}
