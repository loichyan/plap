use std::collections::BTreeMap;

use plap::{Arg, ArgAttrs, Errors, Parser};
use proc_macro2::{Ident, Span, TokenStream};
use syn::parse::{Nothing, ParseStream};
use syn::{Attribute, Data, DeriveInput, Field, GenericArgument, ItemStruct, PathArguments, Type};

use crate::args::{CheckArgs, ContainerCheckArgs};
use crate::dyn_parser::DynParser;

pub fn expand(input: ItemStruct, item: DeriveInput) -> syn::Result<TokenStream> {
    let (groups, check) = crate::args::parse_container_args(&input.attrs)?;
    let mut defs = parse_defs(&input)?;
    defs.extend(groups.into_iter().map(|(k, v)| (k, Def::Group(v))));

    let mut errors = Errors::default();
    Checker {
        c: plap::Checker::default(),
        target: &input.ident,
        check: &check,
        defs: &mut defs,
        errors: &mut errors,
    }
    .check_item(&item)?;

    errors.fail()
}

fn parse_defs(input: &ItemStruct) -> syn::Result<ArgDefs> {
    let mut defs = ArgDefs::default();
    for field in input.fields.iter() {
        let (name, parser) = parse_field(field)?;
        let (arg, check) = crate::args::parse_field_args(&field.attrs)?;
        defs.insert(
            name.clone(),
            Def::Arg(ArgDef {
                i: Arg::from_string(name.to_string()),
                parser,
                attrs: arg.build_arg_attrs()?,
                check,
            }),
        );
    }
    Ok(defs)
}

fn parse_field(field: &Field) -> syn::Result<(&Ident, DynParser)> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| syn_error!(Span::call_site(), "field name is required"))?;
    let parser = infer_arg_type(&field.ty)
        .and_then(DynParser::get)
        .ok_or_else(|| syn_error!(ident.span(), "unsupported type"))?;
    Ok((ident, parser))
}

fn infer_arg_type(ty: &Type) -> Option<&Ident> {
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
                ty.path.get_ident()
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) type ArgDefs = BTreeMap<Ident, Def>;

pub(crate) enum Def {
    Arg(ArgDef),
    Group(GroupDef),
}

pub(crate) struct ArgDef {
    pub i: Arg<Nothing>,
    pub parser: DynParser,
    pub attrs: ArgAttrs,
    pub check: CheckArgs,
}

pub(crate) struct GroupDef {
    pub members: Vec<Ident>,
}

impl Def {
    pub fn as_arg(&self) -> Option<&ArgDef> {
        match self {
            Def::Arg(a) => Some(a),
            Def::Group(_) => None,
        }
    }

    pub fn as_arg_mut(&mut self) -> Option<&mut ArgDef> {
        match self {
            Def::Arg(a) => Some(a),
            Def::Group(_) => None,
        }
    }

    pub fn as_group(&self) -> Option<&GroupDef> {
        match self {
            Def::Arg(_) => None,
            Def::Group(g) => Some(g),
        }
    }
}

struct Checker<'a> {
    c: plap::Checker,
    target: &'a Ident,
    check: &'a ContainerCheckArgs,
    defs: &'a mut ArgDefs,
    errors: &'a mut Errors,
}

impl Checker<'_> {
    fn check_item(&mut self, item: &DeriveInput) -> syn::Result<()> {
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
        // parse defined arguments
        let mut found_any = false;
        for attr in attrs.iter() {
            if let Some(ident) = attr.meta.path().get_ident() {
                if ident == self.target {
                    let r = attr.parse_args_with(|input: ParseStream| {
                        found_any = true;
                        self.c.with_source(ident.span());
                        self.parse_args(input)
                    });
                    self.errors.add_result(r);
                }
            }
        }
        if !found_any {
            return Ok(());
        }

        // perform defined checks
        self.errors
            .add_result(self.check.check(&mut self.c, self.defs));
        for (field, def) in self.defs.iter() {
            if let Some(arg) = def.as_arg() {
                self.errors
                    .add_result(arg.check.check(&mut self.c, self.defs, field));
            }
        }
        self.errors.add_result(self.c.finish());

        // reset
        for def in self.defs.values_mut() {
            if let Some(arg) = def.as_arg_mut() {
                arg.i.clear();
            }
        }
        Ok(())
    }

    fn parse_args(&mut self, input: ParseStream) -> syn::Result<()> {
        let mut parser = Parser::new(input);
        parser.parse_all_with(|parser| {
            let key = parser.peek_key()?;
            if let Some(arg) = self.defs.get_mut(&key).and_then(Def::as_arg_mut) {
                let span = parser.consume_next()?.unwrap();
                parser.next_value_with(arg.attrs.get_kind(), |input| arg.parser.parse(input))?;
                arg.i.add(key, Nothing);
                Ok(Some(span))
            } else {
                Ok(None)
            }
        })
    }
}
