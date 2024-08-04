use plap::{AnyArg, Arg, ArgAttrs, ArgKind, Args, Checker, Parser};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, LitBool, Token};

use crate::define_args::{ArgDefs, GroupDef};

pub(crate) fn parse_container_args(
    attrs: &[Attribute],
) -> syn::Result<(Vec<(Ident, GroupDef)>, ContainerCheckArgs)> {
    let mut group_defs = Vec::default();
    let mut check_args = ContainerCheckArgs::init();
    for attr in attrs.iter() {
        if let Some(key) = attr.meta.path().get_ident() {
            if key == "group" {
                attr.parse_args_with(|input: ParseStream| {
                    Parser::new(input).parse_all_with(|parser| {
                        let name = parser.next_key()?;
                        let mut attrs = ArgAttrs::default();
                        attrs.kind(ArgKind::Expr);
                        let members = parser.next_value::<List<Ident>>(&attrs)?;
                        let span = name.span();
                        group_defs.push((
                            name,
                            GroupDef {
                                members: members.elems.into_iter().collect(),
                            },
                        ));
                        Ok(Some(span))
                    })
                })?;
            } else if key == "check" {
                attr.parse_args_with(|input: ParseStream| {
                    Parser::new(input).parse_all(&mut check_args)
                })?;
            }
        }
    }
    Ok((group_defs, check_args))
}

pub(crate) fn parse_field_args(attrs: &[Attribute]) -> syn::Result<(ArgArgs, CheckArgs)> {
    let mut arg_args = ArgArgs::init();
    let mut check_args = CheckArgs::init();
    for attr in attrs.iter() {
        if let Some(key) = attr.meta.path().get_ident() {
            if key == "arg" {
                attr.parse_args_with(|input: ParseStream| {
                    Parser::new(input).parse_all(&mut arg_args)
                })?;
            } else if key == "check" {
                attr.parse_args_with(|input: ParseStream| {
                    Parser::new(input).parse_all(&mut check_args)
                })?;
            }
        }
    }
    Ok((arg_args, check_args))
}

macro_rules! define_plap_args {
    ($(#[::$attr:meta])*
    #[apply_with($apply_with:ident)]
    $vis:vis struct $name:ident {$(
        $(#[::$f_attr:meta])*
        #[arg($kind:ident)]
        $f_vis:vis $f_name:ident: $f_ty:ty,
    )*}) => {
        ::plap::define_args! {
            $(#[::$attr])*
            $vis struct $name {$(
                $(#[::$f_attr])*
                #[arg($kind)]
                $f_vis $f_name: ::plap::Arg<$f_ty>,
            )*}
        }

        impl $name {
            fn _apply_to(
                &self,
                target: &mut $apply_with::Target,
                ctx: &$apply_with::Context,
            ) -> ::syn::Result<()> {
                $(define_plap_args!(@apply_with($kind) $apply_with)(
                    target,
                    &self.$f_name,
                    ctx,
                    $apply_with::Target::$f_name,
                )?;)*
                Ok(())
            }
        }
    };
    (@apply_with(is_flag) $T:ident) => ($T::apply_flag_to);
    (@apply_with(is_expr) $T:ident) => ($T::apply_expr_to);
}

define_plap_args! {
    #[apply_with(ApplyContainerCheck)]
    pub(crate) struct ContainerCheckArgs {
        #[arg(is_expr)]
        pub exclusive_group: MaybeList<Ident>,
        #[arg(is_expr)]
        pub exclusive_aliases: MaybeList<Ident>,
        #[arg(is_expr)]
        pub required_all: MaybeList<Ident>,
        #[arg(is_expr)]
        pub required_any: MaybeList<Ident>,
        #[arg(is_expr)]
        pub blocked_all: MaybeList<Ident>,
    }
}

#[allow(non_snake_case)]
mod ApplyContainerCheck {
    use super::*;

    pub(super) type Target = Checker;
    pub(super) type Context = ArgDefs;

    pub(super) fn apply_expr_to<'a, T>(
        target: &mut Target,
        arg: &Arg<T>,
        ctx: &'a Context,
        f: impl Fn(&mut Target, T::Type) -> &mut Target,
    ) -> syn::Result<()>
    where
        T: ToAnyArg<'a>,
    {
        for val in arg.values() {
            f(target, val.to_any_arg(ctx)?);
        }
        Ok(())
    }
}

define_plap_args! {
    #[apply_with(ApplyArg)]
    pub(crate) struct ArgArgs {
        #[arg(is_flag)]
        pub is_expr: LitBool,
        #[arg(is_flag)]
        pub is_flag: LitBool,
        #[arg(is_flag)]
        pub is_token_tree: LitBool,
        #[arg(is_flag)]
        pub is_help: LitBool,
        #[arg(is_flag)]
        pub optional: LitBool,
    }
}

#[allow(non_snake_case)]
mod ApplyArg {
    use super::*;

    pub(super) type Target = ArgAttrs;
    pub(super) type Context = ();

    pub(super) fn apply_flag_to(
        target: &mut Target,
        arg: &Arg<LitBool>,
        _ctx: &Context,
        f: impl Fn(&mut Target) -> &mut Target,
    ) -> syn::Result<()> {
        for (key, val) in arg.keys().iter().zip(arg.values()) {
            ensure_is_true(key, val)?;
            f(target);
        }
        Ok(())
    }
}

define_plap_args! {
    #[apply_with(ApplyCheck)]
    pub(crate) struct CheckArgs {
        #[arg(is_flag)]
        pub exclusive: LitBool,
        #[arg(is_flag)]
        pub required: LitBool,
        #[arg(is_expr)]
        pub requires: Ident,
        #[arg(is_expr)]
        pub requires_all: MaybeList<Ident>,
        #[arg(is_expr)]
        pub requires_any: MaybeList<Ident>,
        #[arg(is_expr)]
        pub conflicts_with: Ident,
        #[arg(is_expr)]
        pub conflicts_with_any: MaybeList<Ident>,
        #[arg(is_flag)]
        pub blocked: LitBool,
    }
}

#[allow(non_snake_case)]
mod ApplyCheck {
    use super::*;

    pub(super) type Target = Checker;
    pub(super) struct Context<'a> {
        pub field: &'a Ident,
        pub defs: &'a ArgDefs,
    }

    pub(super) fn apply_flag_to(
        target: &mut Target,
        arg: &Arg<LitBool>,
        ctx: &Context,
        f: impl for<'t> Fn(&'t mut Target, &dyn AnyArg) -> &'t mut Target,
    ) -> syn::Result<()> {
        let a = ctx.field.to_any_arg(ctx.defs)?;
        for (key, val) in arg.keys().iter().zip(arg.values()) {
            ensure_is_true(key, val)?;
            f(target, a);
        }
        Ok(())
    }

    pub(super) fn apply_expr_to<'a, T>(
        target: &mut Target,
        arg: &Arg<T>,
        ctx: &'a Context,
        f: impl for<'t> Fn(&'t mut Target, &dyn AnyArg, T::Type) -> &'t mut Target,
    ) -> syn::Result<()>
    where
        T: ToAnyArg<'a>,
    {
        let a = ctx.field.to_any_arg(ctx.defs)?;
        for b in arg.values() {
            f(target, a, b.to_any_arg(ctx.defs)?);
        }
        Ok(())
    }
}

fn ensure_is_true(k: &Ident, flag: &LitBool) -> syn::Result<()> {
    if flag.value() {
        Ok(())
    } else {
        Err(syn_error!(k.span(), "value cannot be `false`"))
    }
}

impl ContainerCheckArgs {
    pub fn check(&self, checker: &mut Checker, defs: &ArgDefs) -> syn::Result<()> {
        self._apply_to(checker, defs)
    }
}

impl ArgArgs {
    pub fn build_arg_attrs(self) -> syn::Result<ArgAttrs> {
        let mut attrs = ArgAttrs::default();
        self._apply_to(&mut attrs, &())?;
        Ok(attrs)
    }
}

impl CheckArgs {
    pub fn check(&self, checker: &mut Checker, defs: &ArgDefs, field: &Ident) -> syn::Result<()> {
        self._apply_to(checker, &ApplyCheck::Context { field, defs })
    }
}

trait ToAnyArg<'a> {
    type Type;

    fn to_any_arg(&self, defs: &'a ArgDefs) -> syn::Result<Self::Type>;
}

impl<'a> ToAnyArg<'a> for Ident {
    type Type = &'a dyn AnyArg;

    fn to_any_arg(&self, defs: &'a ArgDefs) -> syn::Result<Self::Type> {
        defs.get(self)
            .ok_or_else(|| syn_error!(self.span(), "undefined argument"))
            .and_then(|d| {
                d.as_arg()
                    .ok_or_else(|| syn_error!(self.span(), "group cannot be used as argument"))
            })
            .map(|a| &a.i as &dyn AnyArg)
    }
}

pub(crate) struct List<T> {
    #[allow(dead_code)]
    pub bracket_token: syn::token::Bracket,
    pub elems: Punctuated<T, Token![,]>,
}

impl<T> syn::parse::Parse for List<T>
where
    T: syn::parse::Parse,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            bracket_token: syn::bracketed!(content in input),
            elems: Punctuated::parse_terminated(&content)?,
        })
    }
}

impl<'a> ToAnyArg<'a> for List<Ident> {
    type Type = Vec<&'a dyn AnyArg>;

    fn to_any_arg(&self, defs: &'a ArgDefs) -> syn::Result<Self::Type> {
        self.elems.iter().map(|i| i.to_any_arg(defs)).collect()
    }
}

pub(crate) enum MaybeList<T> {
    Elem(T),
    List(List<T>),
}

impl syn::parse::Parse for MaybeList<Ident> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            input.parse().map(Self::Elem)
        } else if input.peek(syn::token::Bracket) {
            input.parse().map(Self::List)
        } else {
            Err(input.error("expected an identifier or array of identifiers"))
        }
    }
}

impl<'a> ToAnyArg<'a> for MaybeList<Ident> {
    type Type = Vec<&'a dyn AnyArg>;

    fn to_any_arg(&self, defs: &'a ArgDefs) -> syn::Result<Self::Type> {
        match self {
            Self::Elem(i) => defs
                .get(i)
                .ok_or_else(|| syn_error!(i.span(), "undefined group"))
                .and_then(|d| {
                    d.as_group()
                        .ok_or_else(|| syn_error!(i.span(), "argument cannot be used as group"))
                })
                .and_then(|g| g.members.iter().map(|i| i.to_any_arg(defs)).collect()),
            Self::List(l) => l.to_any_arg(defs),
        }
    }
}
