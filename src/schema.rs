#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use std::collections::BTreeMap;
use std::{fmt, ops};

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

impl fmt::Debug for Schema {
    fn fmt<'a>(&'a self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
                    .entries(map.iter().map(|&(i, ref v)| (self.i.id(i), fmt_list(v))))
                    .finish()
            })
        };
        let fmt_arg = |a: &'a ArgInfo| {
            FmtWith(|f| {
                f.debug_struct("Arg")
                    .field("kind", &a.kind)
                    .field("help", &a.help)
                    .finish()
            })
        };
        let fmt_group = |g: &'a GroupInfo| {
            FmtWith(|f| {
                f.debug_struct("Group")
                    .field("members", &fmt_list(&g.members))
                    .finish()
            })
        };
        let fmt_info = |i: &'a Info| {
            FmtWith(|f| match &i.kind {
                InfoKind::None => f.write_str("None"),
                InfoKind::Arg(a) => fmt_arg(a).fmt(f),
                InfoKind::Group(g) => fmt_group(g).fmt(f),
            })
        };
        let fmt_imap = |i: &'a IdMap| {
            FmtWith(|f| {
                f.debug_map()
                    .entries(i.infos.iter().map(|i| (&i.id, fmt_info(i))))
                    .finish()
            })
        };

        f.debug_struct("Schema")
            .field("i", &fmt_imap(&self.i))
            .field("exclusives", &fmt_list(&self.exclusives))
            .field("required", &fmt_list(&self.required))
            .field("requirements", &fmt_map(&self.requirements))
            .field("conflicts", &fmt_map(&self.conflicts))
            .finish()
    }
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
            kind,
            help,
            multiple,
            required,
            requires,
            conflicts_with,
        } = schema;

        let i = self.i.of(id);
        let arg = ArgInfo {
            kind,
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
        debug_assert!(self.exclusives.iter().all(|&t| t != i));
        if !multiple {
            self.exclusives.push(i);
        }

        debug_assert!(self.required.iter().all(|&t| t != i));
        if required {
            self.required.push(i);
        }

        debug_assert!(self.requirements.iter().all(|&(t, _)| t != i));
        if !requires.is_empty() {
            self.requirements
                .push((i, requires.into_iter().map(self.i.by_of()).collect()));
        }

        debug_assert!(self.conflicts.iter().all(|&(t, _)| t != i));
        if !conflicts_with.is_empty() {
            self.conflicts
                .push((i, conflicts_with.into_iter().map(self.i.by_of()).collect()));
        }
    }

    pub(crate) fn id(&self, i: Idx) -> &Id {
        self.i.id(i)
    }

    pub(crate) fn i(&self, id: impl AsRef<str>) -> Option<Idx> {
        self.i.get(id)
    }

    pub(crate) fn require(&self, id: &Id) -> Idx {
        self.i(id)
            .unwrap_or_else(|| panic!("`{}` is not registered", id))
    }

    pub(crate) fn require_arg(&self, i: Idx) -> &ArgInfo {
        self.i.get_info(i).map_or_else(
            || panic!("argument does not exist"),
            |inf| {
                if let InfoKind::Arg(ref inf) = inf.kind {
                    inf
                } else {
                    panic!("`{}` is not registered as an argument", inf.id);
                }
            },
        )
    }

    pub(crate) fn require_group(&self, i: Idx) -> &GroupInfo {
        self.i.get_info(i).map_or_else(
            || panic!("group does not exist"),
            |inf| {
                if let InfoKind::Group(ref inf) = inf.kind {
                    inf
                } else {
                    panic!("`{}` is not registered as a group", inf.id);
                }
            },
        )
    }

    pub fn init_arg<T: ArgParse>(&self, id: impl Into<Id>) -> Arg<T>
    where
        T::Parser: Default,
    {
        self._init_arg(id.into(), <_>::default())
    }

    pub fn init_arg_with<T: ArgParse>(&self, id: impl Into<Id>, parser: T::Parser) -> Arg<T> {
        self._init_arg(id.into(), parser)
    }

    fn _init_arg<T: ArgParse>(&self, id: Id, parser: T::Parser) -> Arg<T> {
        Arg::new(self.require(&id), parser)
    }

    pub fn init_group(&self, id: impl Into<Id>) -> Group {
        self._init_group(id.into())
    }

    fn _init_group(&self, id: Id) -> Group {
        Group::new(self.require(&id))
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
    pub kind: ArgKind,
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
        |i| self.id(i)
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

    pub fn id(&self, i: Idx) -> &Id {
        &self[i].id
    }

    pub fn get(&self, id: impl AsRef<str>) -> Option<Idx> {
        self.ids.get(id.as_ref()).copied()
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
    kind: ArgKind,
    help: String,
    multiple: bool,
    required: bool,
    requires: Vec<Id>,
    conflicts_with: Vec<Id>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgKind {
    Expr,
    Flag,
    TokenTree,
    Help,
}

impl Default for ArgKind {
    fn default() -> Self {
        ArgKind::TokenTree
    }
}

impl ArgSchema {
    pub fn kind(&mut self, kind: ArgKind) -> &mut Self {
        self.kind = kind;
        self
    }

    pub fn is_expr(&mut self) -> &mut Self {
        self.kind(ArgKind::Expr)
    }

    pub fn is_flag(&mut self) -> &mut Self {
        self.kind(ArgKind::Flag)
    }

    pub fn is_token_tree(&mut self) -> &mut Self {
        self.kind(ArgKind::TokenTree)
    }

    pub fn is_help(&mut self) -> &mut Self {
        self.kind(ArgKind::Help)
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

    impl<T: ArgParse> Sealed for Arg<T>
    where
        T::Parser: Default,
    {
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

    impl<T: ArgParse> SchemaFieldType for Arg<T> where T::Parser: Default {}

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
