use std::collections::BTreeMap;

use plap::{Arg, Group, Parser, Schema};
use proc_macro2::{Ident, Span, TokenStream};
use syn::parse::ParseStream;
use syn::{Attribute, Data, DeriveInput, Field, GenericArgument, ItemStruct, PathArguments, Type};

use crate::parser::*;

pub fn expand(attr: ItemStruct, item: DeriveInput) -> syn::Result<TokenStream> {
    let (schema, mut values) = parse_schema(&attr)?;

    let mut parser = Parser::new(&schema);
    for (_, v) in values.iter_mut() {
        match v {
            ValueKind::Arg(a) => {
                parser.add_arg(a);
            }
            ValueKind::Group(g) => {
                parser.add_group(g);
            }
        }
    }

    let mut errors = None;
    Checker {
        parser,
        target: &attr.ident,
        errors: &mut errors,
    }
    .check_input(&item)?;

    // generate a macro for runtime reflection
    let mut tokens = {
        let ident = &attr.ident;
        let debug = format!("{:#?}", schema);
        quote::quote!(
            macro_rules! #ident {
                (@debug) => (#debug);
            }
        )
    };

    // append errors
    if let Some(e) = errors {
        tokens.extend(e.into_compile_error());
    }

    Ok(tokens)
}

struct Checker<'a> {
    parser: Parser<'a>,
    target: &'a Ident,
    errors: &'a mut Option<syn::Error>,
}

impl<'a> Checker<'a> {
    fn check_input(mut self, item: &DeriveInput) -> syn::Result<()> {
        self.check_attrs(&item.attrs)?;
        match &item.data {
            Data::Enum(e) => {
                for variant in e.variants.iter() {
                    self.check_attrs(&variant.attrs)?;
                    self.check_fields(variant.fields.iter())?
                }
            }
            Data::Struct(s) => self.check_fields(s.fields.iter())?,
            Data::Union(u) => self.check_fields(u.fields.named.iter())?,
        }
        Ok(())
    }

    fn check_fields<'f>(&mut self, fields: impl IntoIterator<Item = &'f Field>) -> syn::Result<()> {
        for field in fields {
            self.check_attrs(&field.attrs)?;
        }
        Ok(())
    }

    fn check_attrs(&mut self, attrs: &[Attribute]) -> syn::Result<()> {
        let mut found_any = false;
        for attr in attrs.iter() {
            let ident = if let Some(i) = attr.meta.path().get_ident() {
                i
            } else {
                continue;
            };
            if ident == self.target {
                found_any = true;
                attr.parse_args_with(|input: ParseStream| {
                    self.parser.add_span(ident.span());
                    self.parser.parse(input)
                })?;
            }
        }
        if !found_any {
            return Ok(());
        }

        if let Err(e) = self.parser.finish() {
            if let Some(err) = &mut self.errors {
                err.combine(e);
            } else {
                *self.errors = Some(e);
            }
        }
        self.parser.reset();
        Ok(())
    }
}

fn parse_schema(input: &ItemStruct) -> syn::Result<(Schema, BTreeMap<Box<str>, ValueKind>)> {
    let mut schema = Schema::default();
    let mut values = BTreeMap::default();

    for field in input.fields.iter().map(parse_field) {
        let (field, ident, ty) = field?;
        let id = ident.to_string();
        match ty {
            FieldKind::Arg(p) => {
                schema.register_arg(&id, crate::attrs::parse_arg_schema(&field.attrs)?);
                let arg = schema.init_arg_with(&id, p);
                values.insert(id.into(), ValueKind::Arg(arg));
            }
            FieldKind::Group => {
                schema.register_group(&id, crate::attrs::parse_group_schema(&field.attrs)?);
                let group = schema.init_group(&id);
                values.insert(id.into(), ValueKind::Group(group));
            }
        }
    }

    Ok((schema, values))
}

fn parse_field(field: &Field) -> syn::Result<(&Field, &Ident, FieldKind)> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| syn_error!(Span::call_site(), "tuple struct is not allowed"))?;
    let ty =
        FieldKind::infer(&field.ty).ok_or_else(|| syn_error!(ident.span(), "unsupported type"))?;
    Ok((field, ident, ty))
}

enum ValueKind {
    Arg(Arg<DynValue>),
    Group(Group),
}

enum FieldKind {
    Arg(DynParser),
    Group,
}

impl FieldKind {
    fn infer(ty: &Type) -> Option<Self> {
        match ty {
            Type::Path(ref p) => {
                if p.path.leading_colon.is_some() {
                    return None;
                }
                let ty = p.path.segments.first()?;
                if ty.ident == "Arg" {
                    let arg = if let PathArguments::AngleBracketed(ref a) = ty.arguments {
                        a
                    } else {
                        return None;
                    };
                    let arg = arg.args.first()?;
                    let ty = if let GenericArgument::Type(Type::Path(p)) = arg {
                        p
                    } else {
                        return None;
                    };
                    if ty.qself.is_some() {
                        return None;
                    }
                    let ident = ty.path.get_ident()?;
                    DynParser::get(ident).map(Self::Arg)
                } else if ty.ident == "Group" {
                    if let PathArguments::None = ty.arguments {
                        Some(Self::Group)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
