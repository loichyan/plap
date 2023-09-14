use crate::{
    runtime::{Id, RuntimeBuilder},
    Name, RawName, DUMMY_NAME,
};

pub struct Group {
    id: Id,
}

#[must_use]
pub struct GroupBuilder<'a> {
    id: Id,
    rt: &'a mut RuntimeBuilder,
    state: GroupState,
}

pub(crate) struct GroupState {
    pub name: RawName,
    pub members: Vec<Id>,
    pub required: bool,
    pub multiple: bool,
    pub requires: Vec<Id>,
    pub conflicts: Vec<Id>,
}

impl GroupState {
    pub fn new() -> Self {
        Self {
            name: DUMMY_NAME,
            members: Vec::new(),
            required: false,
            multiple: false,
            conflicts: Vec::new(),
            requires: Vec::new(),
        }
    }
}

impl<'a> GroupBuilder<'a> {
    pub(crate) fn new(id: Id, rt: &'a mut RuntimeBuilder) -> Self {
        GroupBuilder {
            id,
            rt,
            state: GroupState::new(),
        }
    }

    pub fn arg<N>(mut self, name: N) -> Self
    where
        N: Into<Name>,
    {
        self.state.members.push(self.rt.register(name));
        self
    }

    // TODO: pub fn capacity(mut self, capacity: usize) -> Self

    pub fn required(mut self) -> Self {
        self.state.required = true;
        self
    }

    pub fn multiple(mut self) -> Self {
        self.state.multiple = true;
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

    pub fn finish(self) -> Group {
        let Self { id, rt, state, .. } = self;
        rt.finish_group(id, state);
        Group { id }
    }
}

impl Group {
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> Id {
        self.id
    }
}
