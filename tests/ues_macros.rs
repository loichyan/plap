use plap::{define_args, Arg};
use syn::parse::Nothing;
use syn::{Expr, LitBool, LitInt, Type};

define_args! {
    #[::derive(Debug)]
    #[group(grp1 = [arg2, arg5])]
    #[group(grp2 = [arg1, arg3])]
    #[check(exclusive_group = grp1, required_any = grp1)]
    pub struct MyArgs {
        /// Argument #1
        #[arg(is_expr)]
        #[check(exclusive, required)]
        arg1: Arg<Expr>,
        /// Argument #2
        #[arg(is_flag)]
        #[check(exclusive, requires = arg3)]
        arg2: Arg<LitBool>,
        /// Argument #3
        #[arg(is_token_tree)]
        arg3: Arg<Type>,
        /// Argument #4
        #[arg(is_token_tree)]
        #[check(exclusive, conflicts_with_any = grp1)]
        arg4: Arg<Type>,
        /// Argument #5
        #[arg(is_expr)]
        #[check(exclusive)]
        arg5: Arg<LitInt>,
        /// Show usage
        #[arg(is_help)]
        help: Arg<Nothing>,
    }
}

define_args! {
    #[::derive(Debug)]
    pub enum MyArgEnum {
        /// Argument #1
        #[arg(is_expr)]
        arg1(Expr),
        /// Argument #2
        #[arg(is_flag)]
        arg2(LitBool),
        /// Argument #3
        #[arg(is_token_tree)]
        arg3(Type),
        /// Argument #4
        #[arg(is_token_tree)]
        arg4(Type),
        /// Argument #5
        #[arg(is_expr)]
        arg5(LitInt),
        /// Show usage
        #[arg(is_help)]
        help(Nothing),
    }
}
