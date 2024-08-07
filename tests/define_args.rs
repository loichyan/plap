#[plap_macros::define_args {
    #[group(grp1 = [arg2, arg5])]
    #[group(grp2 = [arg1, arg3])]
    #[check(exclusive_group = grp1, required_any = grp1)]
    struct my_arg {
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
        #[check(exclusive, conflicts_with_each = grp1)]
        arg4: Arg<Type>,
        /// Argument #5
        #[arg(is_expr, optional)]
        arg5: Arg<OptionalLitInt>,
        /// Show usage
        #[arg(is_help)]
        help: Arg<Nothing>,
    }
}]
struct UserInput {
    #[my_arg(arg1 = "value #1", arg5 = 1, arg5, arg5)]
    some_field: String,
    #[my_arg(arg1 = "value #2", arg2)]
    #[my_arg(arg3 = "Vec<String>")]
    another_field: i32,
}
