#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(unused)]

use crate::registry::register;
use plap::{Arg, Optional};
use syn::parse::Nothing;
use syn::{Expr, Ident, Lit, LitBool, LitFloat, LitInt, LitStr, Path, Type};

macro_rules! define_args {
        ($(#$attr:tt)* $vis:vis struct $name:ident $body:tt) => {{
            ::plap::define_args!($(#$attr)* $vis struct $name $body);
            $crate::registry::register::<$name>(stringify!($name));
        }};
    }

include!(concat!(env!("OUT_DIR"), "/tests_includes.rs"));
