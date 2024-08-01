#[plap_macros::define_args_slim {
    #[check(exclusive_group = [arg2, arg5])]
    #[check(required_any = [arg2, arg5])]
    struct my_arg {
        /// Argument #1
        #[arg(is_expr)]
        #[check(required, exclusive)]
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
        #[check(conflicts_with_all = [arg2, arg5])]
        arg4: Arg<Type>,
        /// Argument #5
        #[arg(is_expr)]
        arg5: Arg<LitInt>,
        /// Show usage
        #[arg(is_help)]
        help: Arg<Nothing>,
    }
}]
struct UserInput {
    #[my_arg(arg1 = "value #1", arg5 = 1)]
    some_field: String,
    #[my_arg(arg1 = "value #2", arg2 = false)]
    #[my_arg(arg3 = "Vec<String>")]
    another_field: i32,
}
