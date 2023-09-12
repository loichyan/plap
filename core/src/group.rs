use crate::{
    runtime::{Id, RuntimeBuilder},
    Name,
};

pub struct Group {
    id: Id,
}

pub struct GroupBuilder<'a> {
    id: Id,
    rt: &'a mut RuntimeBuilder,
    state: GroupState,
}

pub(crate) struct GroupState {
    pub name: Name,
    pub members: Vec<Id>,
    pub required: bool,
    pub requires: Vec<Id>,
    pub conflicts: Vec<Id>,
}

impl<'a> GroupBuilder<'a> {
    pub(crate) fn new(name: Name, rt: &'a mut RuntimeBuilder) -> Self {
        GroupBuilder {
            id: rt.register(name),
            rt,
            state: GroupState {
                name,
                required: false,
                conflicts: Vec::new(),
                requires: Vec::new(),
                members: Vec::new(),
            },
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

    pub fn arg(mut self, name: Name) -> Self {
        self.state.members.push(self.rt.register(name));
        self
    }

    pub fn finish(self) -> Group {
        let Self { id, rt, state, .. } = self;
        rt.track_group(id, state);
        Group { id }
    }
}

impl Group {
    pub(crate) fn id(&self) -> Id {
        self.id
    }
}
