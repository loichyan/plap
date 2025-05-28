use crate::checker::Checker;
use crate::parser::Parser;
use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;

pub trait Args: Sized {
    fn init() -> Self;

    fn parse_next(&mut self, parser: &mut Parser) -> crate::Result<Span>;

    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut new = Self::init();
        Parser::new(input).parse_all(&mut new)?;
        Ok(new)
    }

    fn check_with(&self, checker: &mut Checker);

    fn check(self) -> syn::Result<Self> {
        let mut checker = Checker::new();
        self.check_with(&mut checker);
        checker.finish()?;
        Ok(self)
    }
}

pub trait ArgEnum: Sized {
    fn name(&self) -> &'static str;

    fn parse_next(parser: &mut Parser) -> crate::Result<(Ident, Self)>;

    fn parse(input: ParseStream) -> syn::Result<Vec<(Ident, Self)>> {
        let mut args = Vec::new();
        Parser::new(input).parse_all_with(|parser| {
            Self::parse_next(parser).map(|(i, a)| {
                let span = i.span();
                args.push((i, a));
                span
            })
        })?;
        Ok(args)
    }
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ArgDesc {
    pub kind: ArgKind,
    pub optional: bool,
}

impl ArgDesc {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kind(&mut self, kind: ArgKind) -> &mut Self {
        self.kind = kind;
        self
    }

    pub fn is_expr(&mut self) -> &mut Self {
        self.kind(ArgKind::Expr)
    }

    pub fn is_flag(&mut self) -> &mut Self {
        self.kind(ArgKind::Flag)
    }

    pub fn is_token_tree(&mut self) -> &mut Self {
        self.kind(ArgKind::TokenTree)
    }

    pub fn is_help(&mut self) -> &mut Self {
        self.kind(ArgKind::Help)
    }

    pub fn optional(&mut self) -> &mut Self {
        self.optional = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
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

#[derive(Debug)]
pub struct Arg<T> {
    #[cfg(feature = "string")]
    name: crate::str::Str,
    #[cfg(not(feature = "string"))]
    name: &'static str,
    keys: Vec<Ident>,
    values: Vec<T>,
}

impl<T> Arg<T> {
    pub fn new(name: &'static str) -> Self {
        #[allow(clippy::useless_conversion)]
        Self {
            #[cfg(feature = "string")]
            name: name.into(),
            #[cfg(not(feature = "string"))]
            name,
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    #[cfg(feature = "string")]
    #[cfg_attr(docsrs, doc(cfg(feature = "string")))]
    pub fn from_string(name: impl Into<String>) -> Self {
        Self {
            name: crate::str::Str::from(name.into()),
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    pub fn name(&self) -> &str {
        #[cfg(feature = "string")]
        return self.name.as_str();
        #[cfg(not(feature = "string"))]
        return self.name;
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn keys(&self) -> &[Ident] {
        &self.keys
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn add(&mut self, key: Ident, value: T) {
        self.keys.push(key);
        self.values.push(value);
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }

    pub fn take_last(&mut self) -> Option<T> {
        self.values.pop()
    }

    pub fn take_one(&mut self) -> T {
        self.take_optional()
            .unwrap_or_else(|| panic!("too few values provided"))
    }

    pub fn take_optional(&mut self) -> Option<T> {
        let val = self.values.pop()?;
        if !self.values.is_empty() {
            panic!("too many values provided");
        }
        Some(val)
    }

    pub fn take_many(&mut self) -> Vec<T> {
        if self.values.is_empty() {
            panic!("too few values provided");
        }
        self.take_any()
    }

    pub fn take_any(&mut self) -> Vec<T> {
        std::mem::take(&mut self.values)
    }
}

impl Arg<syn::LitBool> {
    pub fn take_flag(&mut self) -> bool {
        self.take_flag_or(false)
    }

    pub fn take_flag_or(&mut self, default: bool) -> bool {
        self.take_optional().map(|b| b.value()).unwrap_or(default)
    }
}
