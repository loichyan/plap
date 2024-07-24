use std::collections::BTreeMap;

use plap::ArgParse;
use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;

type DynParserMap = BTreeMap<Ident, fn() -> DynParser>;

thread_local! {
    static DYN_PARSER_MAP: DynParserMap = {
        use syn::*;
        use syn::parse::{Nothing, Parse};

        macro_rules! make_parsers {
            ($($ty:ident),* $(,)?) => {{
                let mut map = DynParserMap::default();
                $(map.insert(
                    Ident::new(stringify!($ty), Span::call_site()),
                    || DynParser(|input| {
                        <$ty as Parse>::parse(input).map(|_| ())
                    }),
                );)*
                map
            }};
        }
        // only a small set of types are supported
        make_parsers![
            DeriveInput,
            Expr,
            GenericArgument,
            Ident,
            Lifetime,
            Lit,
            LitBool,
            LitFloat,
            LitInt,
            LitStr,
            Meta,
            Path,
            Type,
            Visibility,
            WherePredicate,
            Nothing,
        ]
    };
}

#[derive(Clone)]
pub(crate) struct DynParser(fn(ParseStream) -> syn::Result<()>);

impl DynParser {
    pub fn get(ty: &Ident) -> Option<Self> {
        DYN_PARSER_MAP.with(|m| m.get(ty).copied()).map(|f| f())
    }
}

pub(crate) struct DynValue;

impl ArgParse for DynValue {
    type Parser = DynParser;

    fn parse_value(parser: &mut Self::Parser, input: ParseStream) -> syn::Result<Self> {
        (parser.0)(input).map(|_| DynValue)
    }

    fn reset(_: &mut Self::Parser) {}
}
