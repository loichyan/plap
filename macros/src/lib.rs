use syn::parse_macro_input;

#[macro_use]
mod util;
mod args;
mod define_args;
mod dyn_parser;

/// Tests `plap::define_args!` in place.
///
/// ```
/// #[plap_macros::define_args {
///     #[group(grp1 = [arg2, arg5])]
///     #[group(grp2 = [arg1, arg3])]
///     #[check(exclusive_group = grp1, required_any = grp1)]
///     struct my_arg {
///         /// Argument #1
///         #[arg(is_expr)]
///         #[check(exclusive, required)]
///         arg1: Arg<Expr>,
///         /// Argument #2
///         #[arg(is_flag)]
///         #[check(exclusive, requires = arg3)]
///         arg2: Arg<LitBool>,
///         /// Argument #3
///         #[arg(is_token_tree)]
///         arg3: Arg<Type>,
///         /// Argument #4
///         #[arg(is_token_tree)]
///         #[check(exclusive, conflicts_with_all = grp1)]
///         arg4: Arg<Type>,
///         /// Argument #5
///         #[arg(is_expr)]
///         #[check(exclusive)]
///         arg5: Arg<LitInt>,
///         /// Show usage
///         #[arg(is_help)]
///         help: Arg<Nothing>,
///     }
/// }]
/// struct UserInput {
///     #[my_arg(arg1 = "value #1", arg5 = 1)]
///     some_field: String,
///     #[my_arg(arg1 = "value #2", arg2 = false)]
///     #[my_arg(arg3 = "Vec<String>")]
///     another_field: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn define_args(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    define_args::expand(parse_macro_input!(attr as _), parse_macro_input!(item as _))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
