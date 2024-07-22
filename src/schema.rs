#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use std::collections::BTreeMap;

use crate::id::Id;
use crate::parser2::{Arg, ArgGroup, Parser};

pub(crate) type Idx = usize;

#[derive(Debug, Default)]
pub struct Schema {
    ids: IdMap,
    // relation graph
    required: Vec<Idx>,
    requirements: BTreeMap<Idx, Vec<Idx>>,
    conflicts: BTreeMap<Idx, Vec<Idx>>,
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
        self.update_relations(i, required, requires, conflicts_with);

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
            multiple,
        };
        self.ids.register(i, InfoKind::ArgGroup(group));
        self.update_relations(i, required, requires, conflicts_with);

        self
    }

    fn update_relations(
        &mut self,
        i: Idx,
        required: bool,
        requires: Vec<Id>,
        conflicts_with: Vec<Id>,
    ) {
        debug_assert!(!self.required.iter().any(|&t| i == t));
        if required {
            self.required.push(i);
        }

        debug_assert!(!self.requirements.contains_key(&i));
        if !requires.is_empty() {
            self.requirements
                .insert(i, requires.into_iter().map(self.ids.as_map()).collect());
        }

        debug_assert!(!self.conflicts.contains_key(&i));
        if !conflicts_with.is_empty() {
            self.conflicts.insert(
                i,
                conflicts_with.into_iter().map(self.ids.as_map()).collect(),
            );
        }
    }

    pub(crate) fn get_idx(&self, id: impl AsRef<str>) -> Option<Idx> {
        self.ids.get(id.as_ref())
    }

    pub(crate) fn get_info(&self, i: Idx) -> Option<&Info> {
        self.ids.get_info(i)
    }

    pub(crate) fn ensure_arg_registered(&self, i: Idx) {
        self.get_info(i).map_or_else(
            || panic!("argument does not exist"),
            |inf| {
                if !matches!(inf.kind, InfoKind::Arg(_)) {
                    panic!("`{}` is not registered as an argument", inf.id);
                }
            },
        )
    }

    pub(crate) fn ensure_group_registered(&self, i: Idx) {
        self.get_info(i).map_or_else(
            || panic!("group does not exist"),
            |inf| {
                if !matches!(inf.kind, InfoKind::ArgGroup(_)) {
                    panic!("`{}` is not registered as a group", inf.id);
                }
            },
        )
    }

    pub fn init_arg<T>(&self, id: impl Into<Id>) -> Arg<T> {
        self._init_arg(id.into())
    }

    fn _init_arg<T>(&self, id: Id) -> Arg<T> {
        let i = self
            .get_idx(&id)
            .unwrap_or_else(|| panic!("argument `{}` is unregistered", id));
        if is_debug!() {
            self.ensure_arg_registered(i)
        };
        Arg::new(i)
    }

    pub fn init_group(&self, id: impl Into<Id>) -> ArgGroup {
        self._init_group(id.into())
    }

    fn _init_group(&self, id: Id) -> ArgGroup {
        let i = self
            .get_idx(&id)
            .unwrap_or_else(|| panic!("group `{}` is unregistered", id));
        if is_debug!() {
            self.ensure_group_registered(i);
        }
        ArgGroup::new(i)
    }
}

#[derive(Debug, Default)]
pub(crate) struct IdMap {
    ids: BTreeMap<Id, Idx>,
    infos: Vec<Info>,
}

#[derive(Debug)]
pub(crate) struct Info {
    pub id: Id,
    pub kind: InfoKind,
}

#[derive(Debug)]
pub(crate) enum InfoKind {
    Unregistered,
    Arg(ArgInfo),
    ArgGroup(ArgGroupInfo),
}

#[derive(Debug)]
pub(crate) struct ArgInfo {
    pub typ: ArgType,
    pub action: ArgAction,
    pub help: Box<str>,
}

#[derive(Debug)]
pub(crate) struct ArgGroupInfo {
    pub members: Vec<Idx>,
    pub multiple: bool,
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

    pub fn get(&self, id: &str) -> Option<Idx> {
        self.ids.get(id).copied()
    }

    pub fn get_info(&self, i: Idx) -> Option<&Info> {
        self.infos.get(i)
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

pub trait Args {
    fn schema() -> Schema;

    fn init(schema: &Schema) -> Self;

    fn parser<'a>(&'a mut self, schema: &'a Schema) -> Parser<'a>;
}
