use crate::{
    arg::{Arg, ArgBuilder, ArgState},
    Name,
};
use proc_macro2::Span;
use std::collections::{btree_map, BTreeMap};
use syn::Result;

pub struct Runtime {
    names: Vec<Name>,
    states: Vec<ArgState>,
}

pub struct RuntimeBuilder {
    ids: BTreeMap<Name, Id>,
    names: Vec<Name>,
    states: Vec<ArgState>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Id(usize);

impl RuntimeBuilder {
    pub(crate) fn register(&mut self, name: Name) -> Id {
        debug_assert_eq!(self.ids.len(), self.states.len());
        match self.ids.entry(name) {
            btree_map::Entry::Occupied(t) => *t.get(),
            btree_map::Entry::Vacant(t) => {
                let id = Id(self.states.len());
                self.states.push(ArgState::new());
                t.insert(id);
                id
            }
        }
    }

    pub(crate) fn track_state(&mut self, id: Id, state: ArgState) {
        self.states[id.0] = state;
    }

    pub fn arg<T>(&mut self, name: Name) -> ArgBuilder<T> {
        ArgBuilder::new(self.register(name), self)
    }

    pub fn finish(self) -> Runtime {
        let Self { names, states, .. } = self;
        Runtime { names, states }
    }
}

impl Runtime {
    pub(crate) fn name_of(&self, id: Id) -> &str {
        self.names[id.0]
    }

    pub(crate) fn track_source(&mut self, id: Id, span: Span) {
        self.states
            .get_mut(id.0)
            .expect("undefined argument")
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
