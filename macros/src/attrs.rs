use plap::{Arg, ArgKind, ArgParse, ArgSchema, Args, GroupSchema, Parser, Schema};
use proc_macro2::Span;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::{Attribute, LitBool, LitStr, Token};

thread_local! {
    static PLAP_ARGS_SCHEMA: Schema = PlapArgs::schema();
}

pub(crate) fn parse_arg_schema(attrs: &[Attribute]) -> syn::Result<ArgSchema> {
    PLAP_ARGS_SCHEMA
        .with(|schema| PlapArgs::parse_arg_schema(schema, |parser| parse_attrs(parser, attrs)))
}

pub(crate) fn parse_group_schema(attrs: &[Attribute]) -> syn::Result<GroupSchema> {
    PLAP_ARGS_SCHEMA
        .with(|schema| PlapArgs::parse_group_schema(schema, |parser| parse_attrs(parser, attrs)))
}

fn parse_attrs(parser: &mut Parser, attrs: &[Attribute]) -> syn::Result<()> {
    for attr in attrs.iter() {
        let ident = if let Some(i) = attr.meta.path().get_ident() {
            i
        } else {
            continue;
        };
        if ident == "plap" {
            attr.parse_args_with(|input: ParseStream| parser.parse(input))?;
        } else if ident == "doc" {
            if let Some(val) = get_attr_str(attr) {
                let help: &mut Arg<LitStr> = parser.get_arg_mut("help").unwrap();
                help.add_value(ident.span(), val);
            }
        }
    }
    Ok(())
}

fn get_attr_str(attr: &Attribute) -> Option<LitStr> {
    let exp = if let syn::Meta::NameValue(ref m) = attr.meta {
        &m.value
    } else {
        return None;
    };
    let lit = if let syn::Expr::Lit(ref l) = exp {
        &l.lit
    } else {
        return None;
    };
    if let syn::Lit::Str(ref s) = lit {
        // no clone without syn's feature clone-impls
        Some(LitStr::new(&s.value(), s.span()))
    } else {
        None
    }
}

macro_rules! define_plap_args {
    ($(#[$attr:meta])*
    $vis:vis struct $name:ident {$(
            $(#[doc = $doc:literal])*
            #[plap($kind:ident, arg = $arg_cfg:ident, group = $grp_cfg:ident)]
            $f_vis:vis $f_name:ident : $f_ty:ty,
    )*}) => {
        ::plap::define_args! {
            $(#[$attr])*
            $vis struct $name {$(
                $(#[doc = $doc])*
                #[plap($kind, multiple)]
                $f_vis $f_name: ::plap::Arg<$f_ty>,
            )*}
        }

        impl $name {
            fn parse_arg_schema(
                schema: &Schema,
                f: impl FnOnce(&mut Parser) -> ::syn::Result<()>,
            ) -> ::syn::Result<ArgSchema> {
                define_plap_args!(@parse_schema
                    name=[$name]
                    ty=[ArgSchema]
                    fields=[$(($arg_cfg, $kind, $f_name))*]
                    schema, f
                );
            }

            fn parse_group_schema(
                schema: &Schema,
                f: impl FnOnce(&mut Parser) -> ::syn::Result<()>,
            ) -> ::syn::Result<GroupSchema> {
                define_plap_args!(@parse_schema
                    name=[$name]
                    ty=[GroupSchema]
                    fields=[$(($grp_cfg, $kind, $f_name))*]
                    schema, f
                );
            }
        }
    };
    (@parse_schema
        name=[$name:ident]
        ty=[$ty:ident]
        fields=[$(($cfg:ident, $kind:ident, $f_name:ident))*]
        $schema:ident, $f:ident
    ) => {
        let mut args = $name::init($schema);
        let mut parser = $name::init_parser($schema, &mut args);

        // mark unsupported methods
        const _UNSUPPORTED: &'static str =
            concat!("does not exist on `", stringify!($ty), "`");
        $(define_plap_args!(@cfg:not($cfg)
            parser.require_empty_with_msg(stringify!($f_name), _UNSUPPORTED);
        );)*

        // parse
        $f(&mut parser)?;
        parser.finish()?;

        // build schema
        let mut schema = $ty::default();
        $(define_plap_args!(@cfg:$cfg
            define_plap_args!(@apply:$kind)($ty::$f_name, &mut schema, args.$f_name)?;
        );)*
        return Ok(schema);
    };
    (@cfg:true $($tt:tt)*) => ($($tt)*);
    (@cfg:false $($tt:tt)*) => ();
    (@cfg:not(true) $($tt:tt)*) => ();
    (@cfg:not(false) $($tt:tt)*) => ($($tt)*);
    (@apply:is_flag) => (apply_flag_to);
    (@apply:is_expr) => (apply_expr_to);
}

define_plap_args! {
    pub(crate) struct PlapArgs {
        #[plap(is_expr, arg = true, group = false)]
        kind: AttrArgKind,

        #[plap(is_flag, arg = true, group = false)]
        pub is_expr: LitBool,

        #[plap(is_flag, arg = true, group = false)]
        pub is_flag: LitBool,

        #[plap(is_flag, arg = true, group = false)]
        pub is_token_tree: LitBool,

        #[plap(is_flag, arg = true, group = false)]
        pub is_help: LitBool,

        #[plap(is_expr, arg = false, group = true)]
        pub member: LitStr,

        #[plap(is_expr, arg = false, group = true)]
        pub member_all: LitStrList,

        #[plap(is_expr, arg = true, group = true)]
        pub help: LitStr,

        #[plap(is_flag, arg = true, group = true)]
        pub multiple: LitBool,

        #[plap(is_flag, arg = true, group = true)]
        pub required: LitBool,

        #[plap(is_expr, arg = true, group = true)]
        pub requires: LitStr,

        #[plap(is_expr, arg = true, group = true)]
        pub requires_all: LitStrList,

        #[plap(is_expr, arg = true, group = true)]
        pub conflicts_with: LitStr,

        #[plap(is_expr, arg = true, group = true)]
        pub conflicts_with_all: LitStrList,
    }
}

fn apply_flag_to<F, T, P>(mut f: F, schema: &mut T, arg: Arg<P>) -> syn::Result<()>
where
    F: FnMut(&mut T) -> &mut T,
    P: AttrParam<Type = bool>,
{
    for val in arg.values() {
        if val._value() {
            f(schema);
        } else {
            return Err(syn_error!(val._span(), "value cannot be false"));
        }
    }
    Ok(())
}

fn apply_expr_to<F, T, A, P>(mut f: F, schema: &mut T, arg: Arg<P>) -> syn::Result<()>
where
    F: FnMut(&mut T, A) -> &mut T,
    P: AttrParam<Type = A>,
{
    for val in arg.values() {
        f(schema, val._value());
    }
    Ok(())
}

define_value_enum! {
    pub(crate) enum AttrArgKind {
        Expr,
        Flag,
        TokenTree,
        Help,
    }
}

pub(crate) struct LitStrList {
    pub bracket_token: syn::token::Bracket,
    pub elems: Punctuated<LitStr, Token![,]>,
}

impl syn::parse::Parse for LitStrList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: syn::bracketed!(content in input),
            elems: Punctuated::parse_terminated(&content)?,
        })
    }
}

trait AttrParam: ArgParse {
    type Type;

    fn _span(&self) -> Span;

    fn _value(&self) -> Self::Type;
}

impl AttrParam for LitBool {
    type Type = bool;

    fn _span(&self) -> Span {
        self.span()
    }

    fn _value(&self) -> Self::Type {
        self.value()
    }
}

impl AttrParam for LitStr {
    type Type = String;

    fn _span(&self) -> Span {
        self.span()
    }

    fn _value(&self) -> Self::Type {
        self.value()
    }
}

impl AttrParam for AttrArgKind {
    type Type = ArgKind;

    fn _span(&self) -> Span {
        self.span()
    }

    fn _value(&self) -> Self::Type {
        match self {
            Self::Expr(_) => ArgKind::Expr,
            Self::Flag(_) => ArgKind::Flag,
            Self::TokenTree(_) => ArgKind::TokenTree,
            Self::Help(_) => ArgKind::Help,
        }
    }
}

impl AttrParam for LitStrList {
    type Type = Vec<String>;

    fn _span(&self) -> Span {
        self.bracket_token.span.join()
    }

    fn _value(&self) -> Self::Type {
        self.elems.iter().map(LitStr::_value).collect()
    }
}
