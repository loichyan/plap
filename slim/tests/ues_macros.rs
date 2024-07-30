use plap_slim::{define_args, Arg, Checker};
use syn::{LitBool, LitStr};

define_args! {
    #[::derive(Debug)]
    #[check(exclusive_group = [is_expr, help])]
    pub(crate) struct MyArgStruct {
        #[arg(is_flag)]
        #[check(required, requires = help)]
        pub is_expr: Arg<LitBool>,
        #[arg(is_expr)]
        #[check(requires_all = [is_expr, help])]
        pub help: Arg<LitStr>,
    }
}

define_args! {
    #[::derive(Debug)]
    pub(crate) enum MyArgEnum {
        #[arg(is_flag)]
        is_expr(LitBool),
        #[arg(is_expr)]
        help(LitStr),
    }
}

fn _use_checker(arg: &MyArgStruct) -> syn::Result<()> {
    let mut checker = Checker::default();

    checker
        .required(&arg.is_expr)
        .requires(&arg.is_expr, &arg.help);

    checker.finish()
}
