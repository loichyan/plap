use crate::{
    runtime::{Id, RuntimeBuilder},
    Name, RawName, DUMMY_NAME,
};
use proc_macro2::Span;
use std::marker::PhantomData;

pub struct Arg<T> {
    id: Id,
    values: Vec<T>,
}

#[must_use]
pub struct ArgBuilder<'a, T> {
    id: Id,
    rt: &'a mut RuntimeBuilder,
    state: ArgState,
    _marker: PhantomData<T>,
}

pub(crate) struct ArgState {
    pub name: RawName,
    pub action: ArgAction,
    pub required: bool,
    pub requires: Vec<Id>,
    pub conflicts: Vec<Id>,
    pub sources: Vec<Span>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgAction {
    Set,
    Append,
}

impl ArgState {
    pub fn new() -> Self {
        Self {
            name: DUMMY_NAME,
            action: ArgAction::Append,
            required: false,
            requires: Vec::new(),
            conflicts: Vec::new(),
            sources: Vec::new(),
        }
    }
}

impl<'a, T> ArgBuilder<'a, T> {
    pub(crate) fn new(id: Id, rt: &'a mut RuntimeBuilder) -> Self {
        ArgBuilder {
            id,
            rt,
            state: ArgState::new(),
            _marker: PhantomData,
        }
    }

    pub fn action(mut self, action: ArgAction) -> Self {
        self.state.action = action;
        self
    }

    pub fn required(mut self) -> Self {
        self.state.required = true;
        self
    }

    pub fn requires<N>(mut self, name: N) -> Self
    where
        N: Into<Name>,
    {
        self.state.requires.push(self.rt.register(name));
        self
    }

    pub fn conflicts<N>(mut self, name: N) -> Self
    where
        N: Into<Name>,
    {
        self.state.conflicts.push(self.rt.register(name));
        self
    }

    pub fn finish(self) -> Arg<T> {
        let Self { id, rt, state, .. } = self;
        rt.track_arg(id, state);
        Arg {
            id,
            values: Vec::new(),
        }
    }
}

impl<T> Arg<T> {
    pub(crate) fn id(&self) -> Id {
        self.id
    }

    pub(crate) fn add_value(&mut self, val: T) {
        self.values.push(val);
    }

    pub fn any(self) -> Vec<T> {
        self.values
    }

    pub fn at_most_one(self) -> Option<T> {
        assert!(self.values.len() <= 1);
        self.values.into_iter().next()
    }

    pub fn only_one(self) -> T {
        assert!(self.values.len() == 1);
        self.values.into_iter().next().unwrap()
    }

    pub fn at_least_one(self) -> Vec<T> {
        assert!(self.values.len() >= 1);
        self.values
    }
}
