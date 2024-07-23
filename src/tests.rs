use quote::quote;

use super::*;

macro_rules! define_args {
    ($(#[$attr:meta])*
    $vis:vis struct $name:ident {$(
            $(#[doc = $doc:literal])*
            $(#[plap($($plap:ident $(= $plap_val:expr)?),* $(,)?)])*
            $f_vis:vis $f_name:ident : $f_ty:ty,
    )*}) => {
        $(#[$attr])*
        $vis struct $name {$(
            $(#[doc = $doc])*
            $f_vis $f_name : $f_ty,
        )*}

        impl Args for $name {
            #[allow(unused_mut)]
            fn schema() -> Schema {
                let mut schema = Schema::default();
                $(<$f_ty as schema_field_type::Sealed>::register_to(
                    &mut schema,
                    Id::from(stringify!($f_name)),
                    {
                        let mut $f_name = <$f_ty as schema_field_type::Sealed>::Schema::default();
                        $($f_name.help($doc);)*
                        $($($f_name.$plap($($plap_val)*);)*)*
                        $f_name
                    },
                );)*
                schema
            }

            fn init(schema: &Schema) -> Self {
                Self {
                    $($f_name: <$f_ty as schema_field_type::Sealed>::init_from(
                        schema,
                        Id::from(stringify!($f_name)),
                    ),)*
                }
            }

            #[allow(unused_mut)]
            fn init_parser<'a>(&'a mut self, schema: &'a Schema) -> Parser<'a> {
                let mut parser = Parser::new(schema);
                $(<$f_ty as schema_field_type::Sealed>::add_to_parser(
                    &mut parser,
                    &mut self.$f_name,
                );)*
                parser
            }
        }
    };
}

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
        #[plap(multiple, member_all = ["arg1", "grp2"])]
        grp1: ArgGroup,
        /// Group #2
        #[plap(required, member_all = ["arg2", "arg4"])]
        grp2: ArgGroup,
    }
}

#[test]
#[should_panic]
fn test_parse() {
    syn::parse::Parser::parse2(
        |tokens: syn::parse::ParseStream| -> syn::Result<()> {
            let schema = MyArgs::schema();
            let mut my_args = MyArgs::init(&schema);
            let mut parser = my_args.init_parser(&schema);
            parser.parse(tokens)?;
            parser.finish()?;
            panic!("{:#?}\n{:#?}", schema, my_args);
        },
        quote!(
            arg1 = "value1",
            help = 12,
            arg2 = false,
            arg3 = "Value2",
            arg4
        ),
    )
    .unwrap();
}
