use crate::parser::Parser;
use proc_macro2::{Ident, Span};
use syn::parse::ParseStream;

pub trait Args: Sized {
    fn init() -> Self;

    fn parse_next(&mut self, parser: &mut Parser) -> syn::Result<Option<Span>>;

    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut new = Self::init();
        Parser::new(input).parse_all(&mut new)?;
        Ok(new)
    }

    fn check(&self, checker: &mut crate::checker::Checker);
}

pub trait ArgEnum: Sized {
    fn parse_next(parser: &mut Parser) -> syn::Result<Option<(Ident, Self)>>;
}

#[macro_export]
macro_rules! define_args {
    ($($tt:tt)*) => {
        $crate::__define_args_impl!(@head=[] check=[] group=[] $($tt)*);
    }
}

/// NON-PUBLIC API
#[macro_export]
#[doc(hidden)]
macro_rules! __define_args_impl {
    // Extract `#[group]` and `#[check]` attributes from the header
    (@head=$h:tt check=$c:tt group=[$($g:tt)*] #[group$group:tt] $($rest:tt)*) => {
        $crate::__define_args_impl!(@head=$h check=$c group=[$($g)* $group] $($rest)*);
    };
    (@head=$h:tt check=[$($c:tt)*] group=$g:tt #[check$check:tt] $($rest:tt)*) => {
        $crate::__define_args_impl!(@head=$h check=[$($c)* $check] group=$g $($rest)*);
    };
    (@head=[$($h:tt)*] check=$c:tt group=$g:tt #[$attr:meta] $($rest:tt)*) => {
        $crate::__define_args_impl!(@head=[$($h)* #[$attr]] check=$c group=$g $($rest)*);
    };
    (@head=[$($h:tt)*] check=$c:tt group=$g:tt $vis:vis $ty:ident $name:ident { $($body:tt)* }) => {
        $crate::__define_args_impl!(@body=[$ty $($h)* @check=$c @group=$g $vis $name] arg=[] check=[] $($body)*);
    };
    // Extract `#[arg]` attributes from fields/variants
    (@body=$b:tt arg=[$($a:tt)*] check=$c:tt #[arg$arg:tt] $($rest:tt)*) => {
        $crate::__define_args_impl!(@body=$b arg=[$($a)* $arg] check=$c $($rest)*);
    };
    (@body=$b:tt arg=$a:tt check=[$($c:tt)*] #[check$check:tt] $($rest:tt)*) => {
        $crate::__define_args_impl!(@body=$b arg=$a check=[$($c)* $check] $($rest)*);
    };
    (@body=[$($b:tt)*] arg=$a:tt check=$c:tt #[$attr:meta] $($rest:tt)*) => {
        $crate::__define_args_impl!(@body=[$($b)* #[$attr]] arg=$a check=$c $($rest)*);
    };
    (@body=[$($b:tt)*] arg=$a:tt check=$c:tt $vis:vis $name:ident: $ty:ty, $($rest:tt)*) => {
        $crate::__define_args_impl!(@body=[$($b)* @arg=$a @check=$c $vis $name: $ty,] arg=[] check=[] $($rest)*);
    };
    (@body=[$($b:tt)*] arg=$a:tt check=$c:tt $name:ident($ty:ty), $($rest:tt)*) => {
        $crate::__define_args_impl!(@body=[$($b)* @arg=$a @check=$c $name($ty),] arg=[] check=[] $($rest)*);
    };
    (@body=[$($b:tt)*] arg=$_a:tt check=$_c:tt $(,)?) => {
        $crate::__define_args_impl!(@$($b)*);
    };
    // Generate implementations for structs
    (@struct
        $(#[$attr:meta])*
        @check=[$(($($check:ident $(= $check_val:expr)?),* $(,)?))*]
        @group=[$(($($group:ident = $group_val:expr),* $(,)?))*]
        $vis:vis $name:ident
        $(
            $(#[$f_attr:meta])*
            @arg=[$(($($arg:ident $(= $arg_val:expr)?),* $(,)?))*]
            @check=[$(($($f_check:ident $(= $f_check_val:expr)?),* $(,)?))*]
            $f_vis:vis $f_name:ident: $f_ty:ty,
        )*
    ) => {
        $(#[$attr])*
        $vis struct $name {$(
            $(#[$f_attr])* $f_vis $f_name: $f_ty,
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
                parser: &mut $crate::private::Parser,
            ) -> $crate::private::arg::StructParseResult {
                // build argument attributes
                $(let mut $f_name = $crate::private::arg::new_attrs();
                $($($crate::private::ArgAttrs::$arg(&mut $f_name, $($arg_val,)*);)*)*)*

                // look for a matched argument,
                let key = $crate::private::arg::parse_key(parser)?;
                $(if $crate::private::arg::is_key(&key, stringify!($f_name)) {
                    // and then add its parsed value
                    return $crate::private::arg::parse_add_value(
                        parser, &$f_name, key, &mut self.$f_name
                    );
                })*

                // if no match, we return the parsed key as an Err
                return $crate::private::arg::unknown_argument(key);
            }

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
        }
    };
    // Generate implementations for enums
    (@enum
        $(#[$attr:meta])*
        @check=$_c:tt
        @group=$_g:tt
        $vis:vis $name:ident
        $(
            $(#[$v_attr:meta])*
            @arg=[$(($($arg:ident $(= $arg_val:expr)?),* $(,)?))*]
            @check=$_vc:tt
            $v_name:ident($v_ty:ty),
        )*
    ) => {
        $(#[$attr])*
        #[allow(non_camel_case_types)]
        $vis enum $name {$(
            $(#[$v_attr])* $v_name($v_ty),
        )*}

        impl $crate::private::ArgEnum for $name {
            fn parse_next(
                parser: &mut $crate::private::Parser,
            ) -> $crate::private::arg::EnumParseResult<$name> {
                // the parsing process is largely the same as ArgStruct,
                $(let mut $v_name = $crate::private::arg::new_attrs();
                $($($crate::private::ArgAttrs::$arg(&mut $v_name, $($arg_val,)*);)*)*)*

                let key = $crate::private::arg::parse_key(parser)?;
                $(if $crate::private::arg::is_key(&key, stringify!($v_name)) {
                    // except here we return the parsed enum directly
                    return $crate::private::arg::parse_value_into::<_, $name>(
                        parser, &$v_name, key, $name::$v_name
                    );
                })*

                return $crate::private::arg::unknown_argument(key);
            }
        }
    };
}
