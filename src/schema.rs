use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use syn::parse::ParseStream;

use crate::id::Id;

type Idx = usize;

#[derive(Debug, Default)]
pub struct Schema {
    ids: IdMap,
    required: BTreeSet<Idx>,
    /// Mutually exclusive groups
    exclusions: BTreeSet<Idx>,
    requirements: BTreeMap<Idx, BTreeSet<Idx>>,
    conflicts: BTreeMap<Idx, BTreeSet<Idx>>,
}

impl Schema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_arg(&mut self, id: impl Into<Id>, schema: ArgSchema) -> &mut Self {
        self._register_arg(id.into(), schema)
    }

    fn _register_arg(&mut self, id: Id, schema: ArgSchema) -> &mut Self {
        let ArgSchema {
            typ,
            action,
            help,
            required,
            requires,
            conflicts_with,
        } = schema;

        let i = self.ids.of(id);
        let arg = ArgInfo {
            typ,
            action,
            help: help.into(),
        };
        self.ids.register(i, InfoKind::Arg(arg));

        if required {
            self.required.insert(i);
        }
        if !requires.is_empty() {
            self.requirements
                .insert(i, requires.into_iter().map(self.ids.as_map()).collect());
        }
        if !conflicts_with.is_empty() {
            self.conflicts.insert(
                i,
                conflicts_with.into_iter().map(self.ids.as_map()).collect(),
            );
        }

        self
    }

    pub fn register_group(&mut self, id: impl Into<Id>, schema: ArgGroupSchema) -> &mut Self {
        self._register_group(id.into(), schema)
    }

    fn _register_group(&mut self, id: Id, schema: ArgGroupSchema) -> &mut Self {
        let ArgGroupSchema {
            multiple,
            members,
            required,
            requires,
            conflicts_with,
        } = schema;

        let i = self.ids.of(id);
        let group = ArgGroupInfo {
            members: members.into_iter().map(self.ids.as_map()).collect(),
        };
        self.ids.register(i, InfoKind::ArgGroup(group));

        if !multiple {
            self.exclusions.insert(i);
        }
        if required {
            self.required.insert(i);
        }
        if !requires.is_empty() {
            self.requirements
                .insert(i, requires.into_iter().map(self.ids.as_map()).collect());
        }
        if !conflicts_with.is_empty() {
            self.conflicts.insert(
                i,
                conflicts_with.into_iter().map(self.ids.as_map()).collect(),
            );
        }

        self
    }

    pub fn init_arg<T>(&self, id: impl Into<Id>) -> Arg<T> {
        self._init_arg(id.into())
    }

    fn _init_arg<T>(&self, id: Id) -> Arg<T> {
        let i = self
            .ids
            .get(&id)
            .unwrap_or_else(|| panic!("argument `{}` is unregistered", id));
        if is_debug!() {
            self.ids.ensure_arg_registered(i)
        };
        Arg {
            i,
            values: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub fn init_group(&self, id: impl Into<Id>) -> ArgGroup {
        self._init_group(id.into())
    }

    fn _init_group(&self, id: Id) -> ArgGroup {
        let i = self
            .ids
            .get(&id)
            .unwrap_or_else(|| panic!("group `{}` is unregistered", id));
        if is_debug!() {
            self.ids.ensure_group_registered(i);
        }
        ArgGroup { i }
    }
}

#[derive(Debug, Default)]
struct IdMap {
    ids: BTreeMap<Id, Idx>,
    infos: Vec<Info>,
}

#[derive(Debug)]
struct Info {
    id: Id,
    kind: InfoKind,
}

#[derive(Debug)]
enum InfoKind {
    Unregistered,
    Arg(ArgInfo),
    ArgGroup(ArgGroupInfo),
}

impl IdMap {
    pub fn as_map(&mut self) -> impl '_ + FnMut(Id) -> Idx {
        |id| self.of(id)
    }

    pub fn of(&mut self, id: Id) -> Idx {
        debug_assert_eq!(self.ids.len(), self.infos.len());
        if let Some(&i) = self.ids.get(&id) {
            return i;
        }
        let i = self.infos.len();
        self.ids.insert(id.clone(), i);
        self.infos.push(Info {
            id,
            kind: InfoKind::Unregistered,
        });
        i
    }

    pub fn register(&mut self, i: Idx, kind: InfoKind) {
        debug_assert!(!matches!(kind, InfoKind::Unregistered));
        let inf = self
            .infos
            .get_mut(i)
            .unwrap_or_else(|| unreachable!("unknown index"));
        match inf.kind {
            InfoKind::Unregistered => inf.kind = kind,
            _ => panic!("`{}` has been registered", inf.id),
        }
    }

    pub fn get(&self, id: &Id) -> Option<Idx> {
        self.ids.get(id).copied()
    }

    pub fn get_info(&self, i: Idx) -> Option<&Info> {
        self.infos.get(i)
    }

    pub fn ensure_arg_registered(&self, i: Idx) {
        self.get_info(i).map_or_else(
            || panic!("argument does not exist"),
            |inf| {
                if !matches!(inf.kind, InfoKind::Arg(_)) {
                    panic!("`{}` is not registered as an argument", inf.id);
                }
            },
        )
    }

    pub fn ensure_group_registered(&self, i: Idx) {
        self.get_info(i).map_or_else(
            || panic!("group does not exist"),
            |inf| {
                if !matches!(inf.kind, InfoKind::ArgGroup(_)) {
                    panic!("`{}` is not registered as a group", inf.id);
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
    requires: Vec<Id>,
    conflicts_with: Vec<Id>,
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

    pub fn requires(&mut self, id: impl Into<Id>) -> &mut Self {
        self.requires.push(id.into());
        self
    }

    pub fn requires_all<I>(&mut self, ids: impl IntoIterator<Item = I>) -> &mut Self
    where
        I: Into<Id>,
    {
        self.requires.extend(ids.into_iter().map(I::into));
        self
    }

    pub fn conflicts_with(&mut self, id: impl Into<Id>) -> &mut Self {
        self.conflicts_with.push(id.into());
        self
    }

    pub fn conflicts_with_all<I>(&mut self, ids: impl IntoIterator<Item = I>) -> &mut Self
    where
        I: Into<Id>,
    {
        self.conflicts_with.extend(ids.into_iter().map(I::into));
        self
    }
}

#[derive(Debug, Default)]
pub struct ArgGroupSchema {
    members: Vec<Id>,
    multiple: bool,
    required: bool,
    requires: Vec<Id>,
    conflicts_with: Vec<Id>,
}

#[derive(Debug)]
struct ArgGroupInfo {
    members: BTreeSet<Idx>,
}

impl ArgGroupSchema {
    #[doc(hidden)]
    pub fn help(&mut self, _help: &str) -> &mut Self {
        self
    }

    pub fn member(&mut self, id: impl Into<Id>) -> &mut Self {
        self.members.push(id.into());
        self
    }

    pub fn member_all<I>(&mut self, ids: impl IntoIterator<Item = I>) -> &mut Self
    where
        I: Into<Id>,
    {
        self.members.extend(ids.into_iter().map(I::into));
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

    pub fn requires(&mut self, id: impl Into<Id>) -> &mut Self {
        self.requires.push(id.into());
        self
    }

    pub fn requires_all<I>(&mut self, ids: impl IntoIterator<Item = I>) -> &mut Self
    where
        I: Into<Id>,
    {
        self.requires.extend(ids.into_iter().map(I::into));
        self
    }

    pub fn conflicts_with(&mut self, id: impl Into<Id>) -> &mut Self {
        self.conflicts_with.push(id.into());
        self
    }

    pub fn conflicts_with_all<I>(&mut self, ids: impl IntoIterator<Item = I>) -> &mut Self
    where
        I: Into<Id>,
    {
        self.conflicts_with.extend(ids.into_iter().map(I::into));
        self
    }
}

pub struct Parser<'a> {
    s: &'a Schema,
    args: BTreeMap<Idx, &'a mut dyn AnyArg>,
    unknowns: Vec<Span>,
    errors: Vec<syn::Error>,
}

impl<'a> Parser<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        Self {
            s: schema,
            args: BTreeMap::new(),
            unknowns: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn add_arg<T>(&mut self, arg: &'a mut Arg<T>) -> &mut Self
    where
        T: 'static + syn::parse::Parse,
    {
        if is_debug!() {
            self.s.ids.ensure_arg_registered(arg.i);
        }
        self.args.insert(arg.i, arg);
        self
    }

    pub fn add_group(&mut self, group: &'a ArgGroup) -> &mut Self {
        if is_debug!() {
            self.s.ids.ensure_group_registered(group.i);
        }
        self
    }

    pub fn get_arg<T: 'static>(&self, id: impl Into<Id>) -> Option<&Arg<T>> {
        self._get_arg(id.into())
    }

    fn _get_arg<T: 'static>(&self, id: Id) -> Option<&Arg<T>> {
        self.s
            .ids
            .get(&id)
            .and_then(|id| self.args.get(&id))
            .map(|arg| {
                arg.as_any()
                    .downcast_ref()
                    .unwrap_or_else(|| panic!("argument type mismatched"))
            })
    }

    pub fn get_arg_mut<T: 'static>(&mut self, id: impl Into<Id>) -> Option<&mut Arg<T>> {
        self._get_arg_mut(id.into())
    }

    fn _get_arg_mut<T: 'static>(&mut self, id: Id) -> Option<&mut Arg<T>> {
        self.s
            .ids
            .get(&id)
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

        fn register_to(target: &mut Schema, name: Id, schema: Self::Schema);

        fn init_from(schema: &Schema, name: Id) -> Self;

        fn add_to_parser<'a>(parser: &mut Parser<'a>, slf: &'a mut Self);
    }

    impl<T: 'static + syn::parse::Parse> Sealed for Arg<T> {
        type Schema = ArgSchema;

        fn register_to(target: &mut Schema, id: Id, schema: Self::Schema) {
            target.register_arg(id, schema);
        }

        fn init_from(schema: &Schema, name: Id) -> Self {
            schema.init_arg(name)
        }

        fn add_to_parser<'a>(parser: &mut Parser<'a>, slf: &'a mut Self) {
            parser.add_arg(slf);
        }
    }

    impl<T: 'static + syn::parse::Parse> SchemaFieldType for Arg<T> {}

    impl Sealed for ArgGroup {
        type Schema = ArgGroupSchema;

        fn register_to(target: &mut Schema, name: Id, schema: Self::Schema) {
            target.register_group(name, schema);
        }

        fn init_from(schema: &Schema, name: Id) -> Self {
            schema.init_group(name)
        }

        fn add_to_parser<'a>(parser: &mut Parser<'a>, slf: &'a mut Self) {
            parser.add_group(slf);
        }
    }

    impl SchemaFieldType for ArgGroup {}
}

pub trait SchemaFieldType: 'static + schema_field_type::Sealed {}

pub struct Arg<T> {
    i: Idx,
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
    i: Idx,
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
                $(<$f_ty as schema_field_type::Sealed>::register_to(
                    &mut schema,
                    Id::from(stringify!($f_name)),
                    {
                        let mut $f_name = <$f_ty as schema_field_type::Sealed>::Schema::default();
                        $($f_name.help($doc);)*
                        $($($f_name.$attr($($attr_val)*);)*)*
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
            fn parser<'a>(&'a mut self, schema: &'a Schema) -> Parser<'a> {
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
    struct MyArgs {
        /// Argument #1
        #[plap(is_expr, required)]
        arg1: Arg<syn::Ident>,
        /// Argument #2
        #[plap(is_flag, requires = "grp1")]
        arg2: Arg<syn::LitBool>,
        /// Argument #3
        #[plap(is_token_tree, conflicts_with = "arg1")]
        arg3: Arg<syn::LitInt>,
        /// Show usage
        #[plap(is_help)]
        help: Arg<syn::parse::Nothing>,
        /// Group #1
        #[plap(member_all = ["arg1", "arg3"])]
        grp1: ArgGroup,
    }
}
