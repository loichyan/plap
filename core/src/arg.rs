use crate::{
    runtime::{Id, RuntimeBuilder},
    Name,
};
use proc_macro2::Span;
use std::marker::PhantomData;

pub struct Arg<T> {
    id: Id,
    values: Vec<T>,
}

pub struct ArgBuilder<'a, T> {
    id: Id,
    rt: &'a mut RuntimeBuilder,
    state: ArgState,
    _marker: PhantomData<T>,
}

pub(crate) struct ArgState {
    pub required: bool,
    pub requires: Vec<Id>,
    pub conflicts: Vec<Id>,
    pub sources: Vec<Span>,
}

impl ArgState {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            required: false,
            conflicts: Vec::new(),
            requires: Vec::new(),
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

    pub fn required(mut self) -> Self {
        self.state.required = true;
        self
    }

    pub fn requires(mut self, name: Name) -> Self {
        self.state.requires.push(self.rt.register(name));
        self
    }

    pub fn conflicts(mut self, name: Name) -> Self {
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
