use plap::{define_args, Arg};
use syn::parse::Nothing;
use syn::Lit;

define_args! {
    #[group(grp1 = [arg2, arg5])]
    #[group(grp2 = [arg1, arg3])]
    #[check(exclusive_group = grp1, required_any = grp1)]
    pub struct MyArgs {
        /// Argument #1
        #[arg(is_expr)]
        #[check(exclusive, required)]
        arg1: Arg<Lit>,
        /// Argument #2
        #[arg(is_flag)]
        #[check(exclusive, requires = arg3)]
        arg2: Arg<Lit>,
        /// Argument #3
        #[arg(is_token_tree)]
        arg3: Arg<Lit>,
        /// Argument #4
        #[arg(is_token_tree)]
        #[check(exclusive, conflicts_with_each = grp1)]
        arg4: Arg<Lit>,
        /// Argument #5
        #[arg(is_expr)]
        #[check(exclusive)]
        arg5: Arg<Lit>,
        /// Show usage
        #[arg(is_help)]
        help: Arg<Nothing>,
    }
}

define_args! {
    pub enum MyArgEnum {
        /// Argument #1
        #[arg(is_expr)]
        arg1(Lit),
        /// Argument #2
        #[arg(is_flag)]
        arg2(Lit),
        /// Argument #3
        #[arg(is_token_tree)]
        arg3(Lit),
        /// Argument #4
        #[arg(is_token_tree)]
        arg4(Lit),
        /// Argument #5
        #[arg(is_expr)]
        arg5(Lit),
        /// Show usage
        #[arg(is_help)]
        help(Nothing),
    }
}
