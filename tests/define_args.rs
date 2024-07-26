#[plap_macros::define_args {
    struct my_arg {
        /// Argument #1
        #[plap(is_expr, required)]
        arg1: Arg<Expr>,
        /// Argument #2
        #[plap(is_flag, requires = "arg3")]
        arg2: Arg<LitBool>,
        /// Argument #3
        #[plap(is_token_tree, multiple)]
        arg3: Arg<Type>,
        /// Argument #4
        #[plap(is_token_tree, conflicts_with = "grp1")]
        arg4: Arg<Type>,
        /// Argument #5
        #[plap(is_expr)]
        arg5: Arg<LitInt>,
        /// Group #1
        #[plap(required, member_all = ["arg2", "arg5"])]
        grp1: Group,
        /// Group #2
        #[plap(multiple, member_all = ["arg1", "arg3"])]
        grp2: Group,
        /// Show usage
        #[plap(is_help, multiple)]
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

#[test]
fn print_schema() {
    println!("{}", my_arg!(@debug));
}
