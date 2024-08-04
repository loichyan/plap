use std::collections::BTreeMap;

use proc_macro2::{Ident, Span};
use syn::parse::{Parse, ParseStream};

type DynParserMap = BTreeMap<Ident, fn() -> DynParser>;

macro_rules! make_parsers {
    ($($name:ident = $ty:ty,)*) => {{
        let mut map = DynParserMap::default();
        $(map.insert(
            Ident::new(stringify!($name), Span::call_site()),
            || DynParser(|input| {
                <$ty as Parse>::parse(input).map(|_| ())
            }),
        );
        // optional parser
        map.insert(
            Ident::new(concat!("Optional", stringify!($name)), Span::call_site()),
            || DynParser(|input| {
                <::plap::Optional<$ty> as Parse>::parse(input).map(|_| ())
            }),
        );)*
        map
    }};
}
thread_local! {
    static DYN_PARSER_MAP: DynParserMap = {
        // only a small set of types are supported
        make_parsers![
            DeriveInput = syn::DeriveInput,
            Expr = syn::Expr,
            GenericArgument = syn::GenericArgument,
            Ident = syn::Ident,
            Lifetime = syn::Lifetime,
            Lit = syn::Lit,
            LitBool = syn::LitBool,
            LitFloat = syn::LitFloat,
            LitInt = syn::LitInt,
            LitStr = syn::LitStr,
            Meta = syn::Meta,
            Path = syn::Path,
            Type = syn::Type,
            Visibility = syn::Visibility,
            WherePredicate = syn::WherePredicate,
            Nothing = syn::parse::Nothing,
        ]
    };
}

#[derive(Clone)]
pub(crate) struct DynParser(fn(ParseStream) -> syn::Result<()>);

impl DynParser {
    pub fn get(ty: &Ident) -> Option<Self> {
        DYN_PARSER_MAP.with(|m| m.get(ty).copied()).map(|f| f())
    }

    pub fn parse(&self, input: ParseStream) -> syn::Result<()> {
        (self.0)(input)
    }
}
