use syn::parse_macro_input;

#[macro_use]
mod macros;
mod args;
mod attrs;
mod define_args;
mod define_args_slim;
mod dyn_parser;

/// Tests `plap::define_args!` in place.
///
/// ```
/// #[plap_macros::define_args {
///     struct my_arg {
///         /// Argument #1
///         #[plap(is_expr, multiple, required)]
///         arg1: Arg<Expr>,
///         /// Argument #2
///         #[plap(is_flag)]
///         arg2: Arg<LitBool>,
///         /// Argument #3
///         #[plap(is_token_tree, multiple)]
///         arg3: Arg<Type>,
///         /// Argument #3
///         #[plap(is_token_tree, conflicts_with = "grp1")]
///         arg4: Arg<Type>,
///         /// Group #1
///         #[plap(required, member_all = ["arg2", "arg3"])]
///         grp1: Group,
///         /// Show usage
///         #[plap(is_help, multiple)]
///         help: Arg<Nothing>,
///     }
/// }]
/// struct UserInput {
///     #[my_arg(arg1 = "value #1", arg2 = false)]
///     some_field: String,
///     #[my_arg(arg1 = "value #2", arg3 = "Vec<String>")]
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

/// Tests `plap::define_args!` in place.
///
/// ```
/// #[plap_macros::define_args {
///     struct my_arg {
///         /// Argument #1
///         #[plap(is_expr, multiple, required)]
///         arg1: Arg<Expr>,
///         /// Argument #2
///         #[plap(is_flag)]
///         arg2: Arg<LitBool>,
///         /// Argument #3
///         #[plap(is_token_tree, multiple)]
///         arg3: Arg<Type>,
///         /// Argument #3
///         #[plap(is_token_tree, conflicts_with = "grp1")]
///         arg4: Arg<Type>,
///         /// Group #1
///         #[plap(required, member_all = ["arg2", "arg3"])]
///         grp1: Group,
///         /// Show usage
///         #[plap(is_help, multiple)]
///         help: Arg<Nothing>,
///     }
/// }]
/// struct UserInput {
///     #[my_arg(arg1 = "value #1", arg2 = false)]
///     some_field: String,
///     #[my_arg(arg1 = "value #2", arg3 = "Vec<String>")]
///     another_field: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn define_args_slim(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    define_args_slim::expand(parse_macro_input!(attr as _), parse_macro_input!(item as _))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
