use crate::{
    arg::{Arg, ArgBuilder, ArgState},
    group::{GroupBuilder, GroupState},
    Name,
};
use proc_macro2::Span;
use std::collections::{btree_map, BTreeMap};
use syn::Result;

pub struct Runtime {
    names: Vec<Name>,
    states: Vec<State>,
}

pub struct RuntimeBuilder {
    ids: BTreeMap<Name, Id>,
    names: Vec<Name>,
    states: Vec<State>,
}

enum State {
    Arg(ArgState),
    Group(GroupState),
    Undefined,
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
            ids: BTreeMap::new(),
            names: Vec::with_capacity(capacity),
            states: Vec::with_capacity(capacity),
        }
    }

    pub(crate) fn register(&mut self, name: Name) -> Id {
        debug_assert_eq!(self.ids.len(), self.states.len());
        match self.ids.entry(name) {
            btree_map::Entry::Occupied(t) => *t.get(),
            btree_map::Entry::Vacant(t) => {
                let id = Id(self.states.len());
                self.states.push(State::Undefined);
                t.insert(id);
                id
            }
        }
    }

    fn track_state(&mut self, id: Id, state: State) {
        match &mut self.states[id.0] {
            slot @ State::Undefined => *slot = state,
            _ => panic!("duplicated definition for '{}'", self.names[id.0]),
        }
    }

    pub fn arg<T>(&mut self, name: Name) -> ArgBuilder<T> {
        ArgBuilder::new(self.register(name), self)
    }

    pub(crate) fn track_arg(&mut self, id: Id, state: ArgState) {
        self.track_state(id, State::Arg(state));
    }

    pub fn group(&mut self, name: Name) -> GroupBuilder {
        GroupBuilder::new(self.register(name), self)
    }

    pub(crate) fn track_group(&mut self, id: Id, state: GroupState) {
        self.track_state(id, State::Group(state));
    }

    pub fn finish(self) -> Runtime {
        let Self { names, states, .. } = self;
        for (id, state) in states.iter().enumerate() {
            if let State::Undefined = state {
                panic!("missing definition for '{}'", names[id]);
            }
        }
        Runtime { names, states }
    }
}

impl Runtime {
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    pub(crate) fn name_of(&self, id: Id) -> &str {
        self.names[id.0]
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

    pub fn finish(mut self) -> Result<()> {
        self.validate()
    }

    fn validate(&mut self) -> Result<()> {
        todo!()
    }
}
