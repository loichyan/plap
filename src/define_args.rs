use proc_macro2::Ident;
use syn::parse::ParseStream;

use crate::parser::{ArgKind, Parser};

#[macro_export]
macro_rules! define_args {
    ($(#[doc = $doc:literal])*
    $(#[::$attr:meta])*
    $(#[group($($group:ident = $group_val:expr),* $(,)?)])*
    $(#[check($($check:ident $(= $check_val:expr)?),* $(,)?)])*
    $vis:vis struct $name:ident {$(
        $(#[doc = $f_doc:literal])*
        $(#[::$f_attr:meta])*
        $(#[arg($($arg:ident $(= $arg_val:expr)?),* $(,)?)])*
        $(#[check($($f_check:ident $(= $f_check_val:expr)?),* $(,)?)])*
        $f_vis:vis $f_name:ident: $f_ty:ty,
    )*}) => {
        $(#[doc = $doc])*
        $(#[$attr])*
        $vis struct $name {$(
            $(#[doc = $f_doc])*
            $(#[$f_attr])*
            $f_vis $f_name: $f_ty,
        )*}

        #[allow(unused_variables)]
        impl $crate::private::Args for $name {
            fn init() -> $name {
                $name {$(
                    $f_name: $crate::private::Arg::new(stringify!($f_name)),
                )*}
            }

            fn parse_next(
                &mut self,
                input: &mut $crate::private::Parser,
            ) -> $crate::private::arg::StructParseResult {
                // build argument attributes
                $(let mut $f_name = $crate::private::arg::new_attrs();
                $($($crate::private::ArgAttrs::$arg(&mut $f_name, $($arg_val,)*);)*)*)*

                // look for a matched argument,
                let key = $crate::private::arg::parse_key(input)?;
                $(if $crate::private::arg::is_key(&key, stringify!($f_name)) {
                    // and then add its parsed value
                    return $crate::private::arg::parse_add_value(
                        input, &$f_name, key, &mut self.$f_name
                    );
                })*

                // if no match, we return the parsed key as an Err
                return $crate::private::arg::unknown_argument(key);
            }

            $crate::private!(@cfg(feature = "checking")
                fn check(
                    &self,
                    checker: &mut $crate::private::Checker,
                ) {
                    // generate argument variables, which can be referred in #[check(...)]
                    $(let $f_name: &dyn $crate::private::AnyArg = &self.$f_name;)*

                    // generate group variables
                    $($(let $group: &[&dyn $crate::private::AnyArg] = &$group_val;)*)*

                    // add container level checks, including groups, requirements, etc
                    $($($crate::private::Checker::$check(
                        checker,
                        $($check_val,)*
                    );)*)*

                    // add field level checks, where the field is passed as the first parameter
                    $($($($crate::private::Checker::$f_check(
                        checker,
                        $f_name,
                        $($f_check_val,)*
                    );)*)*)*
                }
            );
        }
    };
    ($(#[doc = $doc:literal])*
    $(#[::$attr:meta])*
    $vis:vis enum $name:ident {$(
        $(#[doc = $v_doc:literal])*
        $(#[::$v_attr:meta])*
        $(#[arg($($arg:ident $(= $arg_val:expr)?),* $(,)?)])*
        $v_name:ident($v_ty:ty),
    )*}) => {
        $(#[doc = $doc])*
        $(#[$attr])*
        #[allow(non_camel_case_types)]
        $vis enum $name {$(
            $(#[doc = $v_doc])*
            $(#[$v_attr])*
            $v_name($v_ty),
        )*}

        impl $crate::private::ArgEnum for $name {
            fn parse_next(
                input: &mut $crate::private::Parser,
            ) -> $crate::private::arg::EnumParseResult<$name> {
                // the parsing process is largely the same as ArgStruct,
                $(let mut $v_name = $crate::private::arg::new_attrs();
                $($($crate::private::ArgAttrs::$arg(&mut $v_name, $($arg_val,)*);)*)*)*

                let key = $crate::private::arg::parse_key(input)?;
                $(if $crate::private::arg::is_key(&key, stringify!($v_name)) {
                    // except here we return the parsed enum directly
                    return $crate::private::arg::parse_value_into::<_, $name>(
                        input, &$v_name, key, $name::$v_name
                    );
                })*

                return $crate::private::arg::unknown_argument(key);
            }
        }
    };
}

pub trait Args: Sized {
    fn init() -> Self;

    fn parse_next(&mut self, parser: &mut Parser) -> syn::Result<Result<(), Ident>>;

    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut new = Self::init();
        Parser::new(input).parse_all(&mut new)?;
        Ok(new)
    }

    #[cfg(feature = "checking")]
    #[cfg_attr(docsrs, doc(cfg(feature = "checking")))]
    fn check(&self, checker: &mut crate::checker::Checker);
}

pub trait ArgEnum: Sized {
    fn parse_next(parser: &mut Parser) -> syn::Result<Result<(Ident, Self), Ident>>;
}

#[derive(Debug, Default)]
pub struct ArgAttrs {
    kind: ArgKind,
}

impl ArgAttrs {
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

    pub fn get_kind(&self) -> ArgKind {
        self.kind
    }
}
