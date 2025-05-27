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

    fn check(&self, checker: &mut crate::checker::Checker);
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
        impl $crate::r#priv::Args for $name {
            fn init() -> $name {
                $name {$(
                    $f_name: $crate::r#priv::Arg::new(stringify!($f_name)),
                )*}
            }

            fn parse_next(
                &mut self,
                parser: &mut $crate::r#priv::Parser,
            ) -> $crate::r#priv::StructParseResult {
                // Build argument attributes
                $(let mut $f_name = $crate::r#priv::ArgAttrs::new();
                $($($crate::r#priv::ArgAttrs::$arg(&mut $f_name, $($arg_val,)*);)*)*)*

                // Look for a matched argument,
                let key = parser.peek_key()?;
                $(if &key == stringify!($f_name) {
                    // and then add its parsed value.
                    return $crate::r#priv::parse_args(
                        parser, &$f_name, key, &mut self.$f_name
                    );
                })*

                // Return the parsed key as an Err if no match
                return $crate::r#priv::unknown_argument(key);
            }

            fn check(
                &self,
                checker: &mut $crate::r#priv::Checker,
            ) {
                // Generate argument variables, which can be referred in #[check(...)]
                $(let $f_name: &dyn $crate::r#priv::AnyArg = &self.$f_name;)*

                // Generate group variables
                $($(let $group: &[&dyn $crate::r#priv::AnyArg] = &$group_val;)*)*

                // Add container level checks, including groups, requirements, etc
                $($($crate::r#priv::Checker::$check(
                    checker,
                    $($check_val,)*
                );)*)*

                // Add field level checks, where the field is passed as the first parameter
                $($($($crate::r#priv::Checker::$f_check(
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

        impl $crate::r#priv::ArgEnum for $name {
            fn name(&self) -> &'static str {
                match self {$(
                    $name::$v_name(_) => stringify!($v_name),
                )*}
            }

            fn parse_next(
                parser: &mut $crate::r#priv::Parser,
            ) -> $crate::r#priv::EnumParseResult<$name> {
                // The parsing process is almost the same as ArgStruct,
                $(let mut $v_name = $crate::r#priv::ArgAttrs::new();
                $($($crate::r#priv::ArgAttrs::$arg(&mut $v_name, $($arg_val,)*);)*)*)*

                let key = parser.peek_key()?;
                $(if &key == stringify!($v_name) {
                    // except here we return the parsed enum directly.
                    return $crate::r#priv::parse_args_enum::<$v_ty, $name>(
                        parser, &$v_name, key, $name::$v_name
                    );
                })*

                return $crate::r#priv::unknown_argument(key);
            }
        }
    };
}
