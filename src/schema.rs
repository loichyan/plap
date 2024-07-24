#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use std::collections::BTreeMap;
use std::ops;

use crate::id::Id;
use crate::parser::*;
use crate::util::FmtWith;

pub(crate) type Idx = usize;

#[derive(Default)]
pub struct Schema {
    pub(crate) i: IdMap,
    /// The values of an exclusive argument are duplicated with each other. The
    /// members of an exclusive group conflict with each other.
    pub(crate) exclusives: Vec<Idx>,
    pub(crate) required: Vec<Idx>,
    pub(crate) requirements: Vec<(Idx, Vec<Idx>)>,
    pub(crate) conflicts: Vec<(Idx, Vec<Idx>)>,
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
            help,
            multiple,
            required,
            requires,
            conflicts_with,
        } = schema;

        let i = self.i.of(id);
        let arg = ArgInfo {
            typ,
            help: help.into(),
        };
        self.i.register(i, InfoKind::Arg(arg));
        self.update_relations(i, multiple, required, requires, conflicts_with);

        self
    }

    pub fn register_group(&mut self, id: impl Into<Id>, schema: GroupSchema) -> &mut Self {
        self._register_group(id.into(), schema)
    }

    fn _register_group(&mut self, id: Id, schema: GroupSchema) -> &mut Self {
        let GroupSchema {
            multiple,
            members,
            required,
            requires,
            conflicts_with,
        } = schema;

        let i = self.i.of(id);
        let group = GroupInfo {
            members: members.into_iter().map(self.i.by_of()).collect(),
        };
        self.i.register(i, InfoKind::Group(group));
        self.update_relations(i, multiple, required, requires, conflicts_with);

        self
    }

    fn update_relations(
        &mut self,
        i: Idx,
        multiple: bool,
        required: bool,
        requires: Vec<Id>,
        conflicts_with: Vec<Id>,
    ) {
        debug_assert!(self.exclusives.iter().all(|t| *t != i));
        if !multiple {
            self.exclusives.push(i);
        }

        debug_assert!(self.required.iter().all(|t| *t != i));
        if required {
            self.required.push(i);
        }

        debug_assert!(self.requirements.iter().all(|(t, _)| *t != i));
        if !requires.is_empty() {
            self.requirements
                .push((i, requires.into_iter().map(self.i.by_of()).collect()));
        }

        debug_assert!(self.conflicts.iter().all(|(t, _)| *t != i));
        if !conflicts_with.is_empty() {
            self.conflicts
                .push((i, conflicts_with.into_iter().map(self.i.by_of()).collect()));
        }
    }

    pub(crate) fn ensure_all_registered(&self) {
        assert_eq!(self.i.ids.len(), self.i.infos.len());
        self.i.infos.iter().for_each(|inf| {
            if matches!(inf.kind, InfoKind::None) {
                panic!("`{}` is referred but not registered", inf.id);
            }
        })
    }

    pub(crate) fn ensure_arg(&self, i: Idx) -> &ArgInfo {
        self.i.get_info(i).map_or_else(
            || panic!("argument does not exist"),
            |inf| {
                if let InfoKind::Arg(inf) = &inf.kind {
                    inf
                } else {
                    panic!("`{}` is not registered as an argument", inf.id);
                }
            },
        )
    }

    pub(crate) fn ensure_group(&self, i: Idx) -> &GroupInfo {
        self.i.get_info(i).map_or_else(
            || panic!("group does not exist"),
            |inf| {
                if let InfoKind::Group(inf) = &inf.kind {
                    inf
                } else {
                    panic!("`{}` is not registered as a group", inf.id);
                }
            },
        )
    }

    pub fn init_arg<T>(&self, id: impl Into<Id>) -> Arg<T> {
        self._init_arg(id.into())
    }

    fn _init_arg<T>(&self, id: Id) -> Arg<T> {
        Arg::new(self.i.ensure(&id))
    }

    pub fn init_group(&self, id: impl Into<Id>) -> Group {
        self._init_group(id.into())
    }

    fn _init_group(&self, id: Id) -> Group {
        Group::new(self.i.ensure(&id))
    }
}

impl std::fmt::Debug for Schema {
    fn fmt<'a>(&'a self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_list = |list: &'a [Idx]| {
            FmtWith(|f| {
                f.debug_list()
                    .entries(list.iter().copied().map(self.i.by_id()))
                    .finish()
            })
        };
        let fmt_map = |map: &'a [(Idx, Vec<Idx>)]| {
            FmtWith(|f| {
                f.debug_map()
                    .entries(map.iter().map(|(i, v)| (&self.i[*i].id, fmt_list(v))))
                    .finish()
            })
        };

        f.debug_struct("Schema")
            .field("exclusives", &fmt_list(&self.exclusives))
            .field("required", &fmt_list(&self.required))
            .field("requirements", &fmt_map(&self.requirements))
            .field("conflicts", &fmt_map(&self.conflicts))
            .finish()
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
    None,
    Arg(ArgInfo),
    Group(GroupInfo),
}

#[derive(Debug)]
pub(crate) struct ArgInfo {
    pub typ: ArgType,
    pub help: Box<str>,
}

#[derive(Debug)]
pub(crate) struct GroupInfo {
    pub members: Vec<Idx>,
}

impl IdMap {
    pub fn by_of(&mut self) -> impl '_ + FnMut(Id) -> Idx {
        |id| self.of(id)
    }

    pub fn by_id<'a>(&'a self) -> impl '_ + Fn(Idx) -> &'a Id {
        |i| &self[i].id
    }

    pub fn of(&mut self, id: Id) -> Idx {
        debug_assert_eq!(self.ids.len(), self.infos.len());
        if let Some(i) = self.ids.get(&id) {
            return *i;
        }
        let i = self.ids.len();
        self.ids.insert(id.clone(), i);
        self.infos.push(Info {
            id,
            kind: InfoKind::None,
        });
        i
    }

    pub fn register(&mut self, i: Idx, kind: InfoKind) {
        debug_assert!(!matches!(kind, InfoKind::None));
        let inf = &mut self.infos[i];
        match inf.kind {
            InfoKind::None => inf.kind = kind,
            InfoKind::Arg(_) => panic!("`{}` has been registered as an argument", inf.id),
            InfoKind::Group(_) => panic!("`{}` has been registered as a group", inf.id),
        }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn get(&self, id: impl AsRef<str>) -> Option<Idx> {
        self.ids.get(id.as_ref()).copied()
    }

    pub fn ensure(&self, id: &Id) -> Idx {
        self.get(id)
            .unwrap_or_else(|| panic!("`{}` is not registered", id))
    }

    pub fn get_info(&self, i: Idx) -> Option<&Info> {
        self.infos.get(i)
    }
}

impl ops::Index<Idx> for IdMap {
    type Output = Info;

    fn index(&self, i: Idx) -> &Self::Output {
        &self.infos[i]
    }
}

impl ops::IndexMut<Idx> for IdMap {
    fn index_mut(&mut self, i: Idx) -> &mut Self::Output {
        &mut self.infos[i]
    }
}

#[derive(Debug, Default)]
pub struct ArgSchema {
    typ: ArgType,
    help: String,
    multiple: bool,
    required: bool,
    requires: Vec<Id>,
    conflicts_with: Vec<Id>,
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

    pub fn help(&mut self, help: impl AsRef<str>) -> &mut Self {
        self.help.push_str(help.as_ref().trim());
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

#[derive(Debug, Default)]
pub struct GroupSchema {
    members: Vec<Id>,
    multiple: bool,
    required: bool,
    requires: Vec<Id>,
    conflicts_with: Vec<Id>,
}

impl GroupSchema {
    #[doc(hidden)]
    pub fn help(&mut self, _help: impl AsRef<str>) -> &mut Self {
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

pub(crate) mod schema_field_type {
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

    impl Sealed for Group {
        type Schema = GroupSchema;

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

    impl SchemaFieldType for Group {}
}

pub trait SchemaFieldType: 'static + schema_field_type::Sealed {}
