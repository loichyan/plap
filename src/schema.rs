use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use syn::parse::ParseStream;

type Id = usize;
type Name = Box<str>;

#[derive(Debug, Default)]
pub struct Schema {
    ids: IdMap,
    required: BTreeSet<Id>,
    /// Mutually exclusive groups
    exclusions: BTreeSet<Id>,
    requirements: BTreeMap<Id, BTreeSet<Id>>,
    conflicts: BTreeMap<Id, BTreeSet<Id>>,
}

impl Schema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg(&mut self, name: &str, arg: ArgSchema) -> &mut Self {
        let ArgSchema {
            typ,
            action,
            help,
            required,
            requires,
            conflicts_with,
        } = arg;

        let id = self.ids.of(name);
        let arg = ArgInfo {
            typ,
            action,
            help: help.into(),
        };
        self.ids.register(id, InfoKind::Arg(arg));

        if required {
            self.required.insert(id);
        }
        if !requires.is_empty() {
            self.requirements
                .insert(id, requires.into_iter().map(self.ids.as_map()).collect());
        }
        if !conflicts_with.is_empty() {
            self.conflicts.insert(
                id,
                conflicts_with.into_iter().map(self.ids.as_map()).collect(),
            );
        }

        self
    }

    pub fn group(&mut self, name: &str, group: ArgGroupSchema) -> &mut Self {
        let ArgGroupSchema {
            multiple,
            members,
            required,
            requires,
            conflicts_with,
        } = group;

        let id = self.ids.of(name);
        let group = ArgGroupInfo {
            members: members.into_iter().map(self.ids.as_map()).collect(),
        };
        self.ids.register(id, InfoKind::ArgGroup(group));

        if !multiple {
            self.exclusions.insert(id);
        }
        if required {
            self.required.insert(id);
        }
        if !requires.is_empty() {
            self.requirements
                .insert(id, requires.into_iter().map(self.ids.as_map()).collect());
        }
        if !conflicts_with.is_empty() {
            self.conflicts.insert(
                id,
                conflicts_with.into_iter().map(self.ids.as_map()).collect(),
            );
        }

        self
    }

    pub fn init_arg<T>(&self, name: &str) -> Arg<T> {
        let id = self
            .ids
            .get(name)
            .unwrap_or_else(|| panic!("argument `{}` is unregistered", name));
        if is_debug!() {
            self.ids.ensure_arg_registered(id)
        };
        Arg {
            id,
            values: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub fn init_group(&self, name: &str) -> ArgGroup {
        let id = self
            .ids
            .get(name)
            .unwrap_or_else(|| panic!("group `{}` is unregistered", name));
        if is_debug!() {
            self.ids.ensure_group_registered(id);
        }
        ArgGroup { id }
    }
}

#[derive(Debug, Default)]
struct IdMap {
    ids: BTreeMap<Name, Id>,
    infos: Vec<Info>,
}

#[derive(Debug)]
struct Info {
    name: Name,
    kind: InfoKind,
}

#[derive(Debug)]
enum InfoKind {
    Unregistered,
    Arg(ArgInfo),
    ArgGroup(ArgGroupInfo),
}

impl IdMap {
    pub fn as_map(&mut self) -> impl '_ + FnMut(Name) -> Id {
        |name| self.of(&name)
    }

    pub fn of(&mut self, name: &str) -> Id {
        debug_assert_eq!(self.ids.len(), self.infos.len());
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let name = Name::from(name);
        let id = self.infos.len();
        self.ids.insert(name.clone(), id);
        self.infos.push(Info {
            name,
            kind: InfoKind::Unregistered,
        });
        id
    }

    pub fn register(&mut self, id: Id, kind: InfoKind) {
        debug_assert!(!matches!(kind, InfoKind::Unregistered));
        let i = self
            .infos
            .get_mut(id)
            .unwrap_or_else(|| panic!("unknown identifier"));
        match i.kind {
            InfoKind::Unregistered => i.kind = kind,
            _ => panic!("`{}` has been registered", i.name),
        }
    }

    pub fn get(&self, name: &str) -> Option<Id> {
        self.ids.get(name).copied()
    }

    pub fn get_info(&self, id: Id) -> Option<&Info> {
        self.infos.get(id)
    }

    pub fn ensure_arg_registered(&self, id: Id) {
        self.get_info(id).map_or_else(
            || panic!("argument does not exist"),
            |i| {
                if !matches!(i.kind, InfoKind::Arg(_)) {
                    panic!("`{}` is not registered as an argument", i.name);
                }
            },
        )
    }

    pub fn ensure_group_registered(&self, id: Id) {
        self.get_info(id).map_or_else(
            || panic!("group does not exist"),
            |i| {
                if !matches!(i.kind, InfoKind::ArgGroup(_)) {
                    panic!("`{}` is not registered as a group", i.name);
                }
            },
        )
    }
}

#[derive(Debug, Default)]
pub struct ArgSchema {
    typ: ArgType,
    action: ArgAction,
    help: String,
    required: bool,
    requires: Vec<Name>,
    conflicts_with: Vec<Name>,
}

#[derive(Debug)]
struct ArgInfo {
    typ: ArgType,
    action: ArgAction,
    help: Box<str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgAction {
    Append,
    Set,
}

impl Default for ArgAction {
    fn default() -> Self {
        ArgAction::Append
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgType {
    Expr,
    Flag,
    TokenTree,
    Help,
}

impl Default for ArgType {
    fn default() -> Self {
        ArgType::TokenTree
    }
}

impl ArgSchema {
    pub fn typ(&mut self, typ: ArgType) -> &mut Self {
        self.typ = typ;
        self
    }

    pub fn is_expr(&mut self) -> &mut Self {
        self.typ(ArgType::Expr)
    }

    pub fn is_flag(&mut self) -> &mut Self {
        self.typ(ArgType::Flag)
    }

    pub fn is_token_tree(&mut self) -> &mut Self {
        self.typ(ArgType::TokenTree)
    }

    pub fn is_help(&mut self) -> &mut Self {
        self.typ(ArgType::Help)
    }

    pub fn action(&mut self, action: ArgAction) -> &mut Self {
        self.action = action;
        self
    }

    pub fn help(&mut self, help: &str) -> &mut Self {
        self.help.push_str(help.trim());
        self
    }

    pub fn required(&mut self) -> &mut Self {
        self.required = true;
        self
    }

    pub fn requires(&mut self, name: &str) -> &mut Self {
        self.requires.push(name.into());
        self
    }

    pub fn requires_all<'a, I>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.requires.extend(names.into_iter().map(Name::from));
        self
    }

    pub fn conflicts_with(&mut self, name: &str) -> &mut Self {
        self.conflicts_with.push(name.into());
        self
    }

    pub fn conflicts_with_all<'a, I>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.conflicts_with
            .extend(names.into_iter().map(Name::from));
        self
    }
}

#[derive(Debug, Default)]
pub struct ArgGroupSchema {
    members: Vec<Name>,
    multiple: bool,
    required: bool,
    requires: Vec<Name>,
    conflicts_with: Vec<Name>,
}

#[derive(Debug)]
struct ArgGroupInfo {
    members: BTreeSet<Id>,
}

impl ArgGroupSchema {
    #[doc(hidden)]
    pub fn help(&mut self, _help: &str) -> &mut Self {
        self
    }

    pub fn member(&mut self, name: &str) -> &mut Self {
        self.members.push(name.into());
        self
    }

    pub fn member_all<'a, I>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.members.extend(names.into_iter().map(Name::from));
        self
    }

    pub fn multiple(&mut self) -> &mut Self {
        self.multiple = true;
        self
    }

    pub fn required(&mut self) -> &mut Self {
        self.required = true;
        self
    }

    pub fn requires(&mut self, name: &str) -> &mut Self {
        self.requires.push(name.into());
        self
    }

    pub fn requires_all<'a, I>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.requires.extend(names.into_iter().map(Name::from));
        self
    }

    pub fn conflicts_with(&mut self, name: &str) -> &mut Self {
        self.conflicts_with.push(name.into());
        self
    }

    pub fn conflicts_with_all<'a, I>(&mut self, names: I) -> &mut Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.conflicts_with
            .extend(names.into_iter().map(Name::from));
        self
    }
}

pub struct Parser<'a> {
    i: &'a Schema,
    args: BTreeMap<Id, &'a mut dyn AnyArg>,
    unknowns: Vec<Span>,
    errors: Vec<syn::Error>,
}

impl<'a> Parser<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            i: schema,
            args: BTreeMap::new(),
            unknowns: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn arg<T>(&mut self, arg: &'a mut Arg<T>) -> &mut Self
    where
        T: 'static + syn::parse::Parse,
    {
        if is_debug!() {
            self.i.ids.ensure_arg_registered(arg.id);
        }
        self.args.insert(arg.id, arg);
        self
    }

    pub fn group(&mut self, group: &'a ArgGroup) -> &mut Self {
        if is_debug!() {
            self.i.ids.ensure_group_registered(group.id);
        }
        self
    }

    pub fn get_arg<T: 'static>(&self, name: &str) -> Option<&Arg<T>> {
        self.i
            .ids
            .get(name)
            .and_then(|id| self.args.get(&id))
            .map(|arg| {
                arg.as_any()
                    .downcast_ref()
                    .unwrap_or_else(|| panic!("argument type mismatched"))
            })
    }

    pub fn get_arg_mut<T: 'static>(&mut self, name: &str) -> Option<&mut Arg<T>> {
        self.i
            .ids
            .get(name)
            .and_then(|id| self.args.get_mut(&id))
            .map(|arg| {
                arg.as_any_mut()
                    .downcast_mut()
                    .unwrap_or_else(|| panic!("argument type mismatched"))
            })
    }

    pub fn parse(&mut self, _tokens: ParseStream) {
        todo!()
    }

    pub fn finish(self) -> syn::Result<()> {
        todo!()
    }
}

trait AnyArg {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn spans(&self) -> &[Span];

    fn parse_value(&mut self, span: Span, tokens: ParseStream) -> syn::Result<()>;
}

impl<T: 'static + syn::parse::Parse> AnyArg for Arg<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn spans(&self) -> &[Span] {
        &self.spans
    }

    fn parse_value(&mut self, span: Span, tokens: ParseStream) -> syn::Result<()> {
        self.add_value(span, tokens.parse()?);
        Ok(())
    }
}

mod schema_field_type {
    use super::*;

    pub trait Sealed: 'static {
        type Schema: Default;

        fn register_to(target: &mut Schema, name: &str, schema: Self::Schema);

        fn init_from(schema: &Schema, name: &str) -> Self;

        fn add_to_parser<'a>(parser: &mut Parser<'a>, slf: &'a mut Self);
    }

    impl<T: 'static + syn::parse::Parse> Sealed for Arg<T> {
        type Schema = ArgSchema;

        fn register_to(target: &mut Schema, name: &str, schema: Self::Schema) {
            target.arg(name, schema);
        }

        fn init_from(schema: &Schema, name: &str) -> Self {
            schema.init_arg(name)
        }

        fn add_to_parser<'a>(parser: &mut Parser<'a>, slf: &'a mut Self) {
            parser.arg(slf);
        }
    }

    impl<T: 'static + syn::parse::Parse> SchemaFieldType for Arg<T> {}

    impl Sealed for ArgGroup {
        type Schema = ArgGroupSchema;

        fn register_to(target: &mut Schema, name: &str, schema: Self::Schema) {
            target.group(name, schema);
        }

        fn init_from(schema: &Schema, name: &str) -> Self {
            schema.init_group(name)
        }

        fn add_to_parser<'a>(parser: &mut Parser<'a>, slf: &'a mut Self) {
            parser.group(slf);
        }
    }

    impl SchemaFieldType for ArgGroup {}
}

pub trait SchemaFieldType: 'static + schema_field_type::Sealed {}

pub struct Arg<T> {
    id: Id,
    spans: Vec<Span>,
    values: Vec<T>,
}

impl<T> Arg<T> {
    pub fn schema() -> ArgSchema {
        ArgSchema::default()
    }

    pub fn add_value(&mut self, span: Span, value: T) {
        self.spans.push(span);
        self.values.push(value);
    }
}

pub struct ArgGroup {
    id: Id,
}

impl ArgGroup {
    pub fn schema() -> ArgGroupSchema {
        ArgGroupSchema::default()
    }
}

pub trait Args {
    fn schema() -> Schema;

    fn init(schema: &Schema) -> Self;

    fn parser<'a>(&'a mut self, schema: &'a Schema) -> Parser<'a>;
}

macro_rules! define_args {
    (
        $vis:vis struct
        $name:ident {$(
            $(#[doc = $doc:literal])*
            $(#[plap($($attr:ident $(= $attr_val:expr)?),* $(,)?)])*
            $f_vis:vis $f_name:ident : $f_ty:ty,
        )*}) => {
        $vis struct $name { $($f_vis $f_name : $f_ty,)* }

        impl Args for $name {
            #[allow(unused_mut)]
            fn schema() -> Schema {
                let mut schema = Schema::default();
                $(let mut $f_name = <$f_ty as schema_field_type::Sealed>::Schema::default();
                $($f_name.help($doc);)*
                $($($f_name.$attr($($attr_val)*);)*)*
                <$f_ty as schema_field_type::Sealed>::register_to(&mut schema, stringify!($f_name), $f_name);)*
                schema
            }

            fn init(schema: &Schema) -> Self {
                Self {
                    $($f_name: <$f_ty as schema_field_type::Sealed>::init_from(schema, stringify!($f_name)),)*
                }
            }

            #[allow(unused_mut)]
            fn parser<'a>(&'a mut self, schema: &'a Schema) -> Parser<'a> {
                let mut parser = Parser::new(schema);
                $(<$f_ty as schema_field_type::Sealed>::add_to_parser(&mut parser, &mut self.$f_name);)*
                parser
            }
        }
    };
}

define_args! {
    struct MyArgs {
        /// Argument #1.
        #[plap(is_expr, required)]
        arg1: Arg<syn::Ident>,
        /// Argument #2.
        #[plap(is_flag, requires = "grp1")]
        arg2: Arg<syn::LitBool>,
        /// Argument #3.
        #[plap(is_token_tree, conflicts_with = "arg1")]
        arg3: Arg<syn::LitInt>,
        /// Show usage.
        #[plap(is_help)]
        help: Arg<syn::parse::Nothing>,
        /// Group #1.
        #[plap(member_all = ["arg1", "arg3"])]
        grp1: ArgGroup,
    }
}
