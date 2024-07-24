use quote::quote;

use super::*;
use crate::*;

define_args! {
    #[derive(Debug)]
    struct MyArgs {
        /// Argument #1
        #[plap(is_expr, required)]
        arg1: Arg<syn::Expr>,
        /// Argument #2
        #[plap(is_flag, requires = "grp1")]
        arg2: Arg<syn::LitBool>,
        /// Argument #3
        #[plap(is_token_tree, conflicts_with = "grp1")]
        arg3: Arg<syn::Type>,
        /// Argument #4
        #[plap(is_token_tree, requires = "help")]
        arg4: Arg<syn::Type>,
        /// Show usage
        #[plap(is_help)]
        help: Arg<syn::parse::Nothing>,
        /// Group #1
        #[plap(member_all = ["arg1", "grp2"])]
        grp1: Group,
        /// Group #2
        #[plap(required, member_all = ["arg2", "arg4"])]
        grp2: Group,
    }
}

#[test]
#[should_panic]
fn test_parse() {
    syn::parse::Parser::parse2(
        |input: syn::parse::ParseStream| -> syn::Result<()> {
            let schema = MyArgs::schema();
            let mut my_args = MyArgs::init(&schema);
            let mut parser = my_args.init_parser(&schema);
            parser.parse(input)?;
            parser.finish()?;
            panic!("{:#?}\n{:#?}", schema, my_args);
        },
        quote!(
            arg1 = "value1",
            help = 12,
            arg2 = false,
            arg3 = "Value2",
            arg4 = "Value4",
        ),
    )
    .unwrap();
}
