use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;
use syn::{parenthesized, LitStr, Token};

use crate::arg::*;
use crate::id::*;
use crate::schema::*;
use crate::util::Array;

pub struct Parser<'a> {
    pub(crate) schema: &'a Schema,
    pub(crate) values: Array<Value<'a>>,
    pub(crate) unacceptables: Vec<(Idx, Str)>,
    pub(crate) help_spans: Vec<Span>,
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
        Self {
            schema,
            values: schema
                .infos()
                .iter()
                .map(|_| Value {
                    state: ValueState::None,
                    kind: ValueKind::None,
                })
                .collect(),
            unacceptables: <_>::default(),
            help_spans: <_>::default(),
            errors: <_>::default(),
        }
    }

    pub fn add_span(&mut self, span: Span) -> &mut Self {
        self.errors.add_span(span);
        self
    }

    fn add(&mut self, i: Idx, value: ValueKind<'a>) {
        let val = &mut self.values[i];
        match val.kind {
            ValueKind::None => val.kind = value,
            ValueKind::Arg(..) => panic!("`{}` has been added as an argument", self.schema.id(i)),
            ValueKind::Group(..) => panic!("`{}` has been added as a group", self.schema.id(i)),
        }
    }

    pub fn add_arg<T: ArgParse>(&mut self, arg: &'a mut Arg<T>) -> &mut Self {
        let i = arg.i;
        self.add(i, ValueKind::Arg(arg, self.schema.require_arg(i)));
        self
    }

    pub fn add_group(&mut self, group: &'a mut Group) -> &mut Self {
        let i = group.i;
        self.add(i, ValueKind::Group(group, self.schema.require_group(i)));
        self
    }

    pub fn require_empty(&mut self, id: impl Into<Id>) -> &mut Self {
        self._require_empty(id.into(), "not allowed".into())
    }

    pub fn require_empty_with_msg(&mut self, id: impl Into<Id>, msg: impl Into<Str>) -> &mut Self {
        self._require_empty(id.into(), msg.into())
    }

    fn _require_empty(&mut self, id: Id, msg: Str) -> &mut Self {
        self.unacceptables.push((self.schema.require(&id), msg));
        self
    }

    pub fn has(&self, id: impl Into<Id>) -> bool {
        self.schema.i(id.into()).is_some()
    }

    pub fn get_arg<T: ArgParse>(&self, id: impl Into<Id>) -> Option<&Arg<T>> {
        self._get_arg(id.into())
    }

    fn _get_arg<T: ArgParse>(&self, id: Id) -> Option<&Arg<T>> {
        self.schema.i(id).and_then(|i| {
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
        self.schema.i(id).and_then(|i| {
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
            } else if input.parse::<Option<Token![,]>>()?.is_some() {
                // successfully parse an argument
                continue;
            } else if !input.is_empty() {
                self.errors.add(syn_error!(input.span(), "expected a `,`"));
            } else {
                break;
            }

            // consume all input till the next comma
            while input.parse::<Option<Token![,]>>()?.is_none() && !input.is_empty() {
                input.parse::<proc_macro2::TokenTree>()?;
            }
        }
        Ok(())
    }

    fn parse_next(&mut self, input: ParseStream) -> syn::Result<()> {
        let ident = input
            .parse::<Option<Ident>>()?
            .ok_or_else(|| syn_error!(input.span(), "expected an identifier"))?;
        let span = ident.span();

        let (arg, inf) = self
            .schema
            .i(ident.to_string())
            .and_then(|i| {
                if let ValueKind::Arg(ref mut arg, inf) = self.values[i].kind {
                    Some((arg, inf))
                } else {
                    None
                }
            })
            .ok_or_else(|| syn_error!(span, "unknown argument"))?;

        match inf.kind {
            ArgKind::Expr | ArgKind::Flag => {
                if input.parse::<Option<Token![=]>>()?.is_some() {
                    arg.parse_value(span, input)?;
                } else if input.peek(syn::token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    arg.parse_value(span, &content)?;
                } else if inf.kind == ArgKind::Flag && is_eoa(input) {
                    parse_value_from_str(*arg, span, "true")?;
                } else {
                    return Err(syn_error!(span, "expected `= <value>` or `(<value>)`"));
                }
            }
            ArgKind::TokenTree => {
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
            ArgKind::Help => {
                parse_value_from_str(*arg, span, "")?;
                self.help_spans.push(span);
            }
        }

        Ok(())
    }

    pub fn finish(&mut self) -> syn::Result<()> {
        crate::validate::validate(self)
    }

    pub fn reset(&mut self) {
        for v in self.values.iter_mut() {
            v.state = ValueState::None;
            match &mut v.kind {
                ValueKind::None => {}
                ValueKind::Arg(a, _) => a.reset(),
                ValueKind::Group(g, _) => g.reset(),
            }
        }
        self.unacceptables.clear();
        self.help_spans.clear();
        self.errors.reset();
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
    type Parser;

    fn parse_value(parser: &mut Self::Parser, input: ParseStream) -> syn::Result<Self>;

    fn reset(parser: &mut Self::Parser);
}

#[derive(Debug, Default)]
pub struct SynParser;

impl<T: 'static + syn::parse::Parse> ArgParse for T {
    type Parser = SynParser;

    fn parse_value(_: &mut Self::Parser, input: ParseStream) -> syn::Result<Self> {
        input.parse()
    }

    fn reset(_: &mut Self::Parser) {}
}
