mod includes;
mod registry;

fn test_impl(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let name = syn::parse::<syn::Ident>(attr)?;
    let item = syn::parse::<syn::ItemStruct>(item)?;

    let mut args = registry::get(&name.to_string()).ok_or_else(|| {
        syn::Error::new(
            name.span(),
            format!("required definition '{name}' is not found"),
        )
    })?;
    let mut checker = plap::Checker::new();
    let mut errors = plap::Errors::default();

    // Check the specified arguments definition on each field
    for field in item.fields.iter() {
        for attr in field
            .attrs
            .iter()
            .filter(|a| a.path().get_ident().is_some_and(|i| i == &name))
        {
            let r = attr.parse_args_with(|input: syn::parse::ParseStream| {
                plap::Parser::new(input).parse_all_with(|p| args.parse_next(p))
            });
            errors.add_result(r);
        }
        args.check_with(&mut checker);
        args.reset();
        errors.add_result(checker.finish());
    }

    errors.fail()?;
    Ok(<_>::default())
}

#[proc_macro_attribute]
pub fn test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    includes::init();
    test_impl(attr, item)
        .map(<_>::into)
        .map_err(syn::Error::into_compile_error)
        .unwrap_or_else(<_>::into)
}
