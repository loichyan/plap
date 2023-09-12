use crate::{
    arg::{Arg, ArgAction, ArgBuilder, ArgState},
    error::{DefaultFormatter, DelegatedFormatter, Error, ErrorFormatter, ErrorKind, ErrorKind::*},
    group::{GroupBuilder, GroupState},
    Name,
};
use proc_macro2::Span;
use std::{
    collections::{btree_map, BTreeMap},
    fmt,
};
use syn::{Error as SynError, Result};

pub struct Runtime {
    node: Span,
    formatter: DelegatedFormatter,
    states: Vec<State>,
}

pub struct RuntimeBuilder {
    node: Option<Span>,
    namespace: Option<Name>,
    formatter: Option<Box<dyn ErrorFormatter>>,
    ids: BTreeMap<Name, Id>,
    states: Vec<State>,
}

enum State {
    Undefined(UndefinedState),
    Arg(ArgState),
    Group(GroupState),
}

struct UndefinedState {
    name: Name,
}

impl State {
    pub fn name(&self) -> &str {
        match self {
            Self::Arg(ArgState { name, .. })
            | Self::Group(GroupState { name, .. })
            | Self::Undefined(UndefinedState { name, .. }) => name,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Id(usize);

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            node: None,
            namespace: None,
            formatter: None,
            ids: BTreeMap::new(),
            states: Vec::with_capacity(capacity),
        }
    }

    pub fn node(mut self, node: Span) -> Self {
        self.node = Some(node);
        self
    }

    pub fn namespace(mut self, namespace: Name) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub fn formatter<F>(mut self, formatter: F) -> Self
    where
        F: 'static + ErrorFormatter,
    {
        self.formatter = Some(Box::new(formatter));
        self
    }

    pub(crate) fn register(&mut self, name: Name) -> Id {
        debug_assert_eq!(self.ids.len(), self.states.len());
        match self.ids.entry(name) {
            btree_map::Entry::Occupied(t) => *t.get(),
            btree_map::Entry::Vacant(t) => {
                let id = Id(self.states.len());
                self.states.push(State::Undefined(UndefinedState { name }));
                t.insert(id);
                id
            }
        }
    }

    fn track_state(&mut self, id: Id, state: State) {
        match &mut self.states[id.0] {
            slot @ State::Undefined(_) => *slot = state,
            _ => panic!("duplicate definition for '{}'", state.name()),
        }
    }

    pub fn arg<T>(&mut self, name: Name) -> ArgBuilder<T> {
        ArgBuilder::new(name, self)
    }

    pub(crate) fn track_arg(&mut self, id: Id, state: ArgState) {
        self.track_state(id, State::Arg(state));
    }

    pub fn group(&mut self, name: Name) -> GroupBuilder {
        GroupBuilder::new(name, self)
    }

    pub(crate) fn track_group(&mut self, id: Id, state: GroupState) {
        self.track_state(id, State::Group(state));
    }

    pub fn finish(self) -> Runtime {
        let Self {
            node,
            namespace,
            formatter,
            ids: _,
            states,
        } = self;
        for state in states.iter() {
            if let State::Undefined(UndefinedState { name }) = state {
                panic!("missing definition for '{}'", name);
            }
        }
        Runtime {
            node: node.unwrap_or_else(|| Span::call_site()),
            states,
            formatter: formatter
                .map(DelegatedFormatter::Other)
                .unwrap_or_else(|| DelegatedFormatter::Default(DefaultFormatter { namespace })),
        }
    }
}

impl Runtime {
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    pub(crate) fn track_source(&mut self, id: Id, span: Span) {
        self.states
            .get_mut(id.0)
            .and_then(|s| if let State::Arg(s) = s { Some(s) } else { None })
            .expect("given id does not belong to current runtime")
            .sources
            .push(span);
    }

    pub fn track_arg<T>(&mut self, arg: &mut Arg<T>, span: Span, val: T) {
        self.track_source(arg.id(), span);
        arg.add_value(val);
    }

    pub fn finish(self) -> Result<()> {
        RuntimeChecker {
            rt: &self,
            supplied: vec![None; self.states.len()],
            buffer: Vec::new(),
            error: None,
        }
        .check()
    }

    fn state(&self, id: Id) -> &State {
        &self.states[id.0]
    }

    fn visit_args<'a>(&'a self, state: &'a State, mut f: impl FnMut(&'a ArgState)) {
        match state {
            State::Arg(arg) => f(arg),
            State::Group(grp) => self.visit_members(grp, f),
            _ => unreachable!(),
        }
    }

    fn visit_members<'a>(&'a self, grp: &'a GroupState, mut f: impl FnMut(&'a ArgState)) {
        fn visit_members_impl<'a, F>(rt: &'a Runtime, grp: &'a GroupState, f: &mut F)
        where
            F: FnMut(&'a ArgState),
        {
            for id in grp.members.iter().copied() {
                match &rt.state(id) {
                    State::Arg(arg) => f(arg),
                    State::Group(grp) => visit_members_impl(rt, grp, f),
                    _ => unreachable!(),
                }
            }
        }
        visit_members_impl(self, grp, &mut f)
    }

    fn fmt_error(&self, err: &Error) -> SynError {
        SynError::new(err.node(), DisplayError(&self.formatter, err))
    }

    fn check_supplied(&self, id: Id) -> bool {
        match &self.state(id) {
            State::Arg(arg) => !arg.sources.is_empty(),
            State::Group(grp) => grp.members.iter().all(|id| self.check_supplied(*id)),
            _ => unreachable!(),
        }
    }
}

struct RuntimeChecker<'a> {
    rt: &'a Runtime,
    supplied: Vec<Option<bool>>,
    buffer: Vec<&'a str>,
    error: Option<SynError>,
}

impl<'a> RuntimeChecker<'a> {
    pub fn check(mut self) -> Result<()> {
        let rt = self.rt;
        for (id, state) in rt.states.iter().enumerate() {
            let id = Id(id);
            match state {
                State::Arg(arg) => {
                    // A required argument must be supplied.
                    if arg.required && !self.supplied(id) {
                        self.missing_argument(&[rt.node], id);
                    }
                    // Validatevalue count.
                    if matches!(arg.action, ArgAction::Set) && arg.sources.len() > 1 {
                        for node in arg.sources.iter().copied() {
                            self.error(node, DuplicateValue);
                        }
                    }
                    // Check for missing requirements.
                    for id in arg.requires.iter().copied() {
                        if self.supplied(id) {
                            continue;
                        }
                        self.missing_argument(&arg.sources, id);
                    }
                    // Check for conflicting arguments.
                    for id in arg.conflicts.iter().copied() {
                        if !self.supplied(id) {
                            continue;
                        }
                        self.conflicting_argument(&arg.sources, id);
                    }
                }
                State::Group(grp) => {
                    // All members of a required group must be supplied.
                    if grp.required && !self.supplied(id) {
                        self.missing_argument(&[rt.node], id);
                    }
                    if !grp.multiple {
                        // Members in same group conflict with each other.
                        for i in 0..grp.members.len() {
                            let id = grp.members[i];
                            for conflicting in grp.members[..i]
                                .iter()
                                .chain(grp.members[i + 1..].iter())
                                .copied()
                            {
                                if !self.supplied(conflicting) {
                                    continue;
                                }
                                rt.visit_args(rt.state(id), |arg| {
                                    self.conflicting_argument(&arg.sources, conflicting);
                                });
                            }
                        }
                    }
                    // Check for missing requirements.
                    for id in grp.requires.iter().copied() {
                        if self.supplied(id) {
                            continue;
                        }
                        rt.visit_members(grp, |arg| {
                            self.missing_argument(&arg.sources, id);
                        });
                    }
                    // Check for conflicting arguments.
                    for id in grp.conflicts.iter().copied() {
                        if !self.supplied(id) {
                            continue;
                        }
                        rt.visit_members(grp, |arg| {
                            self.conflicting_argument(&arg.sources, id);
                        });
                    }
                }
                _ => unreachable!(),
            }
        }

        self.error.map(Err).unwrap_or(Ok(()))
    }

    fn supplied(&mut self, id: Id) -> bool {
        match &mut self.supplied[id.0] {
            Some(s) => *s,
            slot => {
                let s = self.rt.check_supplied(id);
                *slot = Some(s);
                s
            }
        }
    }

    fn map_members<'b, T>(&'b mut self, state: &State, f: impl FnOnce(&'b [&'a str]) -> T) -> T {
        self.buffer.clear();
        self.rt.visit_args(state, |arg| self.buffer.push(arg.name));
        f(&self.buffer)
    }

    fn error(&mut self, node: Span, kind: ErrorKind) {
        let err = Error::new(node, kind);
        self.error_syn(SynError::new(node, self.rt.fmt_error(&err)));
    }

    fn error_syn(&mut self, err: SynError) {
        if let Some(error) = self.error.as_mut() {
            error.combine(err);
        } else {
            self.error = Some(err);
        }
    }

    fn missing_argument(&mut self, nodes: &[Span], id: Id) {
        let rt = self.rt;
        for node in nodes.iter().copied() {
            let err = self.map_members(rt.state(id), |args| {
                rt.fmt_error(&Error::new(node, MissingArgument { args }))
            });
            self.error_syn(err);
        }
    }

    fn conflicting_argument(&mut self, nodes: &[Span], id: Id) {
        let rt = self.rt;
        for node in nodes.iter().copied() {
            let err = self.map_members(rt.state(id), |args| {
                rt.fmt_error(&Error::new(node, ConflictingArgument { args }))
            });
            self.error_syn(err);
        }
    }
}

struct DisplayError<'a, F>(&'a F, &'a Error<'a>);

impl<F> fmt::Display for DisplayError<'_, F>
where
    F: ErrorFormatter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f, self.1)
    }
}
