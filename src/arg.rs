use std::any::Any;

use proc_macro2::Ident;
use syn::parse::ParseStream;

use crate::parser::*;
use crate::schema::*;

#[derive(Debug)]
pub struct Arg<T: ArgParse> {
    pub(crate) i: Idx,
    parser: T::Parser,
    keys: Vec<Ident>,
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
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    pub fn add(&mut self, key: Ident, value: T) {
        self.keys.push(key);
        self.values.push(value);
    }

    pub fn keys(&self) -> &[Ident] {
        &self.keys
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn parser(&self) -> &T::Parser {
        &self.parser
    }

    pub fn parser_mut(&mut self) -> &mut T::Parser {
        &mut self.parser
    }

    pub fn take_last(mut self) -> Option<T> {
        self.values.pop()
    }

    pub fn take_one(mut self) -> T {
        let val = self
            .values
            .pop()
            .unwrap_or_else(|| panic!("too many values provided"));
        if !self.values.is_empty() {
            panic!("too many values provided");
        }
        val
    }

    pub fn take_many(self) -> Vec<T> {
        if self.values.is_empty() {
            panic!("too few values provided");
        }
        self.values
    }

    pub fn take_any(self) -> Vec<T> {
        self.values
    }

    pub fn reset(&mut self) {
        T::reset(&mut self.parser);
        self.keys.clear();
        self.values.clear();
    }
}

/// A type earsed and object safe [`Arg<T>`].
pub(crate) trait AnyArg {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn keys(&self) -> &[Ident];

    fn parse_value(&mut self, key: Ident, input: ParseStream) -> syn::Result<()>;

    fn reset(&mut self);
}

impl<T: ArgParse> AnyArg for Arg<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn keys(&self) -> &[Ident] {
        &self.keys
    }

    fn parse_value(&mut self, key: Ident, input: ParseStream) -> syn::Result<()> {
        let val = T::parse_value(&mut self.parser, input)?;
        self.add(key, val);
        Ok(())
    }

    fn reset(&mut self) {
        Arg::reset(self)
    }
}

#[derive(Debug)]
pub struct Group {
    pub(crate) i: Idx,
    pub(crate) n: usize,
}

impl Group {
    pub fn schema() -> GroupSchema {
        GroupSchema::default()
    }

    pub(crate) fn new(i: Idx) -> Self {
        Self { i, n: 0 }
    }

    pub fn reset(&mut self) {
        self.n = 0;
    }
}
