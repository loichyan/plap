use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use proc_macro2::Span;
use syn::Result;

use crate::util::{Buffer, Either, Errors};
use crate::{ArgAction, Name, ParserContext};

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
    error: Errors,
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
        self.error.add(e);
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
            spans
                .iter()
                .for_each(|span| error.add(syn::Error::new(*span, &msg)));
        };
        let to_name = |id: &Id| *names.get(id).expect("undefined `Id`");
        let supplied = |id: &Id| sources.get(id).is_some();
        let flat_group = _flat_group(|id| {
            if let Some(members) = groups.get(id) {
                // Flatten a group (assume no nested groups).
                Either::Left(members.iter())
            } else {
                // This is an argument.
                Either::Right(std::iter::once(id))
            }
        });
        // Convert Id to Name and ignore absent arguments.
        let to_sources =
            _to_sources(|id| sources.get(id).map(|spans| (to_name(id), spans.as_slice())));
        let flat_key_group = _flag_key_group(|(this, those)| {
            // Flatten group members from keys,
            flat_group(this)
                // get present sources,
                .filter_map(to_sources)
                // and flatten values.
                .flat_map(|s| those.iter().map(move |that| (s, that)))
        });

        // Ensure the number of values meets the specified action.
        actions
            .iter()
            .filter_map(|(this, action)| {
                to_sources(this).map(|(this, spans)| (this, spans, action))
            })
            .for_each(|(this, spans, action)| match action {
                ArgAction::Set if spans.len() > 1 => add_error(spans, Error::DuplicateArg { this }),
                _ => {}
            });

        // Ensure all required arguments/groups are supplied.
        let node = [context.node()];
        // required by this node
        required
            .iter()
            .map(|that| ((None, &node as &[Span]), that))
            // required by an argument
            .chain(
                requirements
                    .iter()
                    .flat_map(flat_key_group)
                    .map(|((this, spans), that)| ((Some(this), spans), that)),
            )
            .for_each(|((this, spans), that)| {
                if let Some(members) = groups.get(that) {
                    // An argument requires any of a member in a group.
                    if !members.iter().any(supplied) {
                        add_error(
                            spans,
                            Error::MissingRequired {
                                this,
                                required: &*buffer.acquire(members.iter().map(to_name)),
                            },
                        );
                    }
                } else if !supplied(that) {
                    add_error(
                        spans,
                        Error::MissingRequired {
                            this,
                            required: &[to_name(that)],
                        },
                    );
                }
            });

        // Arguments in a group conflict with each other.
        for (_, members) in groups.iter() {
            members
                .iter()
                .enumerate()
                .filter_map(|(i, this)| to_sources(this).map(|s| (i, s)))
                .for_each(|(i, (this, spans))| {
                    members[0..i]
                        .iter()
                        .chain(members[i + 1..].iter())
                        .filter(|id| supplied(*id))
                        .map(to_name)
                        .for_each(|conflict| {
                            add_error(spans, Error::ArgConflict { this, conflict });
                        });
                });
        }

        // Ensure all conflict arguments/groups are not supplied.
        conflicts
            .iter()
            .flat_map(flat_key_group)
            // Flatten values (an argument conflicts with each argument in a group).
            .flat_map(|(s, that)| flat_group(that).map(move |that| (s, that)))
            .for_each(|((this, spans), that)| {
                add_error(
                    spans,
                    Error::ArgConflict {
                        this,
                        conflict: to_name(that),
                    },
                );
            });

        // Reports unexpected arguments/groups.
        unexpected
            .iter()
            // Every argument in a group is unexpected.
            .flat_map(flat_group)
            .filter_map(to_sources)
            .for_each(|(this, spans)| add_error(spans, Error::UnexpectedArg { this }));

        error.take().map_or(Ok(()), Err)
    }
}

fn _to_sources<'a, F>(f: F) -> F
where
    F: Fn(&'a Id) -> Option<(&'a str, &'a [Span])>,
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

fn _flag_key_group<'a, I, F>(f: F) -> F
where
    I: 'a + Iterator<Item = ((&'a str, &'a [Span]), &'a Id)>,
    F: Fn((&'a Id, &'a Vec<Id>)) -> I,
{
    f
}
