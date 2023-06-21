use crate::{ExprArg, FlagArg, NamedArg};
use syn::{
    parse::{Parse, ParseBuffer},
    Result,
};

pub trait ParseStreamExt {
    fn parse_named_arg<T: Parse>(&self) -> Result<T>;
    fn parse_expr_arg<T: Parse>(&self) -> Result<T>;
    fn parse_flag_arg(&self) -> Result<bool>;
}

impl ParseStreamExt for ParseBuffer<'_> {
    fn parse_named_arg<T: Parse>(&self) -> Result<T> {
        self.parse::<NamedArg<T>>().map(|t| t.value)
    }

    fn parse_expr_arg<T: Parse>(&self) -> Result<T> {
        self.parse::<ExprArg<T>>().map(|t| t.value)
    }

    fn parse_flag_arg(&self) -> Result<bool> {
        self.parse::<FlagArg>().map(|_| true)
    }
}
