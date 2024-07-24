use std::any::Any;

use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;
use syn::{parenthesized, LitStr, Token};

use crate::id::Id;
use crate::schema::*;

#[derive(Debug)]
pub struct Arg<T: ArgParse> {
    i: Idx,
    parser: T::Parser,
    spans: Vec<Span>,
    values: Vec<T>,
}

impl<T: ArgParse> Arg<T> {
    pub fn schema() -> ArgSchema {
        ArgSchema::default()
    }

    pub(crate) fn new(i: Idx, parser: T::Parser) -> Self {
        Self {
            i,
            parser,
            spans: <_>::default(),
            values: <_>::default(),
        }
    }

    pub fn add_value(&mut self, span: Span, value: T) {
        self.spans.push(span);
        self.values.push(value);
    }

    pub fn spans(&self) -> &[T] {
        &self.values
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }
}

#[derive(Debug)]
pub struct Group {
    i: Idx,
}

impl Group {
    pub fn schema() -> GroupSchema {
        GroupSchema::default()
    }

    pub(crate) fn new(i: Idx) -> Self {
        Self { i }
    }
}

pub struct Parser<'a> {
    pub(crate) schema: &'a Schema,
    pub(crate) values: Vec<Value<'a>>,
    pub(crate) errors: crate::util::Errors,
}

pub(crate) struct Value<'a> {
    pub state: ValueState,
    pub kind: ValueKind<'a>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ValueState {
    None,
    Busy,
    Empty,
    Provided,
    ProvidedMany,
}

pub(crate) enum ValueKind<'a> {
    None,
    Arg(&'a mut dyn AnyArg, &'a ArgInfo),
    Group(&'a mut Group, &'a GroupInfo),
}

impl<'a> Parser<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        if is_debug!() {
            schema.ensure_all_registered();
        }
        Self {
            schema,
            values: std::iter::repeat_with(|| Value {
                state: ValueState::None,
                kind: ValueKind::None,
            })
            .take(schema.i.len())
            .collect(),
            errors: <_>::default(),
        }
    }

    fn add(&mut self, i: Idx, value: ValueKind<'a>) {
        let val = &mut self.values[i];
        match val.kind {
            ValueKind::None => val.kind = value,
            ValueKind::Arg(..) => panic!("`{}` has been added as an argument", self.schema.i[i].id),
            ValueKind::Group(..) => panic!("`{}` has been added as a group", self.schema.i[i].id),
        }
    }

    pub fn add_arg<T>(&mut self, arg: &'a mut Arg<T>) -> &mut Self
    where
        T: 'static + syn::parse::Parse,
    {
        let i = arg.i;
        self.add(i, ValueKind::Arg(arg, self.schema.ensure_arg(i)));
        self
    }

    pub fn add_group(&mut self, group: &'a mut Group) -> &mut Self {
        let i = group.i;
        self.add(i, ValueKind::Group(group, self.schema.ensure_group(i)));
        self
    }

    pub fn with_span(&mut self, span: Span) -> &mut Self {
        self.errors.set_span(span);
        self
    }

    pub fn get_arg<T: ArgParse>(&self, id: impl Into<Id>) -> Option<&Arg<T>> {
        self._get_arg(id.into())
    }

    fn _get_arg<T: ArgParse>(&self, id: Id) -> Option<&Arg<T>> {
        self.schema.i.get(&id).and_then(|i| {
            if let ValueKind::Arg(arg, _) = &self.values[i].kind {
                arg.as_any().downcast_ref()
            } else {
                None
            }
        })
    }

    pub fn get_arg_mut<T: ArgParse>(&mut self, id: impl Into<Id>) -> Option<&mut Arg<T>> {
        self._get_arg_mut(id.into())
    }

    fn _get_arg_mut<T: ArgParse>(&mut self, id: Id) -> Option<&mut Arg<T>> {
        self.schema.i.get(&id).and_then(|i| {
            if let ValueKind::Arg(arg, _) = &mut self.values[i].kind {
                arg.as_any_mut().downcast_mut()
            } else {
                None
            }
        })
    }

    pub fn parse(&mut self, input: ParseStream) -> syn::Result<()> {
        loop {
            if input.is_empty() {
                break;
            }

            if let Err(e) = self.parse_next(input) {
                self.errors.add(e);
            }

            // consume all input till the next comma
            while input.parse::<Option<Token![,]>>()?.is_none() && !input.is_empty() {
                input.parse::<proc_macro2::TokenTree>()?;
            }
        }
        Ok(())
    }

    fn parse_next(&mut self, input: ParseStream) -> syn::Result<()> {
        let span = input.span();
        let ident = input.parse::<Ident>()?.to_string();

        let (arg, inf) = self
            .schema
            .i
            .get(&ident)
            .and_then(|i| {
                if let ValueKind::Arg(arg, inf) = &mut self.values[i].kind {
                    Some((arg, inf))
                } else {
                    None
                }
            })
            .ok_or_else(|| syn_error!(span, "unknown argument"))?;

        match inf.typ {
            ArgType::Expr | ArgType::Flag => {
                if input.parse::<Option<Token![=]>>()?.is_some() {
                    arg.parse_value(span, input)?;
                } else if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    arg.parse_value(span, &content)?;
                } else if inf.typ == ArgType::Flag && is_eoa(input) {
                    parse_value_from_str(*arg, span, "true")?;
                } else {
                    return Err(syn_error!(span, "expected `= <value>` or `(<value>)`"));
                }
            }
            ArgType::TokenTree => {
                if input.parse::<Option<Token![=]>>()?.is_some() {
                    let content = input.parse::<syn::LitStr>()?;
                    parse_value_from_literal(*arg, span, content)?;
                } else if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    arg.parse_value(span, &content)?;
                } else {
                    return Err(syn_error!(span, "expected `= \"<value>\"` or `(<value>)`"));
                }
            }
            ArgType::Help => {
                parse_value_from_str(*arg, span, "")?;
                self.errors.add_info(span, &inf.help);
            }
        }

        Ok(())
    }

    pub fn finish(self) -> syn::Result<()> {
        crate::validate::validate(self)
    }
}

fn is_eoa(input: ParseStream) -> bool {
    input.peek(Token![,]) || input.is_empty()
}

fn parse_value_from_str(a: &mut dyn AnyArg, span: Span, input: &str) -> syn::Result<()> {
    parse_value_from_literal(a, span, LitStr::new(input, span))
}

fn parse_value_from_literal(a: &mut dyn AnyArg, span: Span, input: LitStr) -> syn::Result<()> {
    input.parse_with(|input: ParseStream| a.parse_value(span, input))
}

pub trait ArgParse: 'static + Sized {
    type Parser: Default;

    fn parse_value(parser: &mut Self::Parser, input: ParseStream) -> syn::Result<Self>;
}

#[derive(Debug, Default)]
pub struct SynParser;

impl<T: 'static + syn::parse::Parse> ArgParse for T {
    type Parser = SynParser;

    fn parse_value(_: &mut Self::Parser, input: ParseStream) -> syn::Result<Self> {
        input.parse()
    }
}

/// A type earsed and object safe [`Arg<T>`].
pub(crate) trait AnyArg {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn spans(&self) -> &[Span];

    fn parse_value(&mut self, span: Span, input: ParseStream) -> syn::Result<()>;
}

impl<T: ArgParse> AnyArg for Arg<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn spans(&self) -> &[Span] {
        &self.spans
    }

    fn parse_value(&mut self, span: Span, input: ParseStream) -> syn::Result<()> {
        let val = T::parse_value(&mut self.parser, input)?;
        self.add_value(span, val);
        Ok(())
    }
}
