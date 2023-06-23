use crate::{
    util::{Buffer, Either, ErrorCollector},
    ArgAction, Name, ParserContext,
};
use proc_macro2::Span;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use syn::Result;

pub(crate) type Rt = Rc<RefCell<Runtime>>;

/// An identifier for [`Arg`].
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Id(usize);

#[derive(Default)]
pub(crate) struct Runtime {
    // registered arguments/groups
    ids: BTreeMap<Name, Id>,
    names: BTreeMap<Id, Name>,
    groups: BTreeMap<Id, Vec<Id>>,
    // source spans
    sources: BTreeMap<Id, Vec<Span>>,
    // saved errors
    error: ErrorCollector,
    // runtime constraints
    actions: BTreeMap<Id, ArgAction>,
    required: BTreeSet<Id>,
    requirements: BTreeMap<Id, Vec<Id>>,
    conflicts: BTreeMap<Id, Vec<Id>>,
    unexpected: Vec<Id>,
}

impl Runtime {
    pub fn register(&mut self, name: Name) -> Id {
        use std::collections::btree_map::Entry;

        let next_id = Id(self.ids.len());
        match self.ids.entry(name) {
            Entry::Occupied(t) => *t.get(),
            Entry::Vacant(t) => {
                t.insert(next_id);
                self.names.insert(next_id, name);
                next_id
            }
        }
    }

    pub fn add_action(&mut self, this: Id, action: ArgAction) {
        self.actions.insert(this, action);
    }

    pub fn add_required(&mut self, this: Id) {
        self.required.insert(this);
    }

    pub fn add_conflicts_with(&mut self, this: Id, conflict: Name) {
        let conflict = self.register(conflict);
        self.conflicts.entry(this).or_default().push(conflict);
        self.conflicts.entry(conflict).or_default().push(this);
    }

    pub fn add_requires(&mut self, this: Id, required: Name) {
        let requirement = self.register(required);
        self.requirements.entry(this).or_default().push(requirement);
    }

    pub fn add_to_group(&mut self, this: Id, group: Name) {
        let group = self.register(group);
        self.groups.entry(group).or_default().push(this);
    }

    pub fn add_member(&mut self, group: Id, arg: Name) {
        let arg = self.register(arg);
        self.groups.entry(group).or_default().push(arg);
    }

    pub fn add_unexpected(&mut self, this: Id) {
        self.unexpected.push(this);
    }

    pub fn add_source(&mut self, this: Id, span: Span) {
        self.sources.entry(this).or_default().push(span);
    }

    pub fn add_error(&mut self, e: syn::Error) {
        self.error.combine(e);
    }

    pub fn finish(self, context: ParserContext) -> Result<()> {
        use crate::Error;

        let Self {
            ids: _,
            names,
            groups,
            sources,
            mut error,
            actions,
            required,
            requirements,
            conflicts,
            unexpected,
        } = self;

        // helpers
        let mut buffer = Buffer::<&str>::default();
        let mut add_error = |spans: &[Span], err: crate::Error| {
            let msg = context.format(&err);
            for span in spans {
                error.combine(syn::Error::new(*span, &msg));
            }
        };
        let to_name = |id: &Id| *names.get(&id).expect("undefined `Id`");
        let supplied = |id: &Id| sources.get(id).is_some();
        let with_spans = _with_spans(|id| sources.get(id).map(|spans| (id, spans.as_slice())));
        let flat_group = _flat_group(|id| {
            if let Some(members) = groups.get(id) {
                // this is a group,
                // assumes no nested groups
                Either::Left(members.iter())
            } else {
                // this is an argument
                Either::Right(std::iter::once(id))
            }
        });
        let flat_pair = _flat_pair(|(this, those)| {
            flat_group(this)
                // skip not supplied arguments
                .filter_map(with_spans)
                // flatten the list of ids
                .flat_map(|(this, spans)| those.iter().map(move |that| (this, spans, that)))
        });

        // Ensure the number of values meet the specified action.
        for (this, spans, action) in actions
            .iter()
            .filter_map(|(this, action)| sources.get(this).map(|spans| (this, spans, action)))
        {
            match action {
                ArgAction::Set if spans.len() > 1 => {
                    add_error(
                        spans,
                        Error::DuplicateArg {
                            this: to_name(this),
                        },
                    );
                }
                _ => {}
            }
        }

        // Ensure all required arguments/groups are supplied.
        let node = [context.node()];
        for (this, spans, that) in std::iter::empty()
            // required by this node
            .chain(required.iter().map(|that| (None, &node as &[Span], that)))
            // required by an argument
            .chain(
                requirements
                    .iter()
                    .flat_map(flat_pair)
                    .map(|(this, spans, that)| (Some(this), spans, that)),
            )
        {
            if let Some(members) = groups.get(that) {
                if !members.iter().all(supplied) {
                    add_error(
                        spans,
                        Error::MissingRequired {
                            this: this.map(to_name),
                            required: &*buffer.acquire(members.iter().map(to_name)),
                        },
                    );
                }
            } else if !supplied(&that) {
                add_error(
                    spans,
                    Error::MissingRequired {
                        this: this.map(to_name),
                        required: &[to_name(that)],
                    },
                );
            }
        }

        // Arguments in a group conflict with each other.
        for (_, members) in groups.iter() {
            for (i, this, spans) in members
                .iter()
                .enumerate()
                .filter_map(|(i, this)| sources.get(this).map(|spans| (i, this, spans)))
            {
                let this = to_name(this);
                for conflict in members[0..i]
                    .iter()
                    .chain(members[i + 1..].iter())
                    .filter(|id| supplied(*id))
                    .map(to_name)
                {
                    add_error(spans, Error::ArgConflict { this, conflict });
                }
            }
        }

        // Ensure all conflict arguments/groups are not supplied.
        for (this, spans, that) in conflicts.iter().flat_map(flat_pair) {
            let this = to_name(&this);
            for that in flat_group(that).filter(|id| supplied(*id)) {
                add_error(
                    spans,
                    Error::ArgConflict {
                        this,
                        conflict: to_name(that),
                    },
                );
            }
        }

        // Reports unexpected arguments/groups.
        for (this, spans) in unexpected
            .iter()
            .flat_map(flat_group)
            .filter_map(with_spans)
        {
            add_error(
                spans,
                Error::UnexpectedArg {
                    this: to_name(this),
                },
            );
        }

        error.take().map_or(Ok(()), Err)
    }
}

fn _with_spans<'a, F>(f: F) -> F
where
    F: Fn(&'a Id) -> Option<(&'a Id, &'a [Span])>,
{
    f
}

fn _flat_group<'a, I, F>(f: F) -> F
where
    I: 'a + Iterator<Item = &'a Id>,
    F: Fn(&'a Id) -> I,
{
    f
}

fn _flat_pair<'a, I, F>(f: F) -> F
where
    I: 'a + Iterator<Item = (&'a Id, &'a [Span], &'a Id)>,
    F: Fn((&'a Id, &'a Vec<Id>)) -> I,
{
    f
}
