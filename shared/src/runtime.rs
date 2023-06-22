use crate::{ArgAction, Name, ParserContext};
use proc_macro2::Span;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ops,
    rc::Rc,
};
use syn::Result;

pub(crate) type Rt = Rc<RefCell<Runtime>>;

/// An identifier for [`Arg`].
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct Id(usize);

#[derive(Default)]
pub(crate) struct Runtime {
    ids: BTreeMap<Name, Id>,
    names: BTreeMap<Id, Name>,
    action: BTreeMap<Id, ArgAction>,
    required: BTreeSet<Id>,
    conflicts_with: BTreeMap<Id, Vec<Id>>,
    requires: BTreeMap<Id, Vec<Id>>,
    group: BTreeMap<Id, Vec<Id>>,
    source: BTreeMap<Id, Vec<Span>>,
    error: ErrorCollector,
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

    pub fn add_action(&mut self, id: Id, action: ArgAction) {
        self.action.insert(id, action);
    }

    pub fn add_required(&mut self, id: Id) {
        self.required.insert(id);
    }

    pub fn add_conflicts_with(&mut self, id: Id, conflict: Name) {
        let conflict = self.register(conflict);
        self.conflicts_with.entry(id).or_default().push(conflict);
        self.conflicts_with.entry(conflict).or_default().push(id);
    }

    pub fn add_requires(&mut self, id: Id, requirement: Name) {
        let requirement = self.register(requirement);
        self.requires.entry(id).or_default().push(requirement);
    }

    pub fn add_group(&mut self, id: Id, group: Name) {
        let group = self.register(group);
        self.requires.entry(group).or_default().push(id);
    }

    pub fn add_source(&mut self, id: Id, span: Span) {
        self.source.entry(id).or_default().push(span);
    }

    pub fn add_error(&mut self, e: syn::Error) {
        self.error.combine(e);
    }

    pub fn finish(self, context: ParserContext) -> Result<()> {
        use crate::Error;

        let Self {
            names,
            source,
            mut error,
            action,
            required,
            mut conflicts_with,
            requires,
            group,
            ..
        } = self;
        let mut buffer = Buffer::<&str> {
            inner: <_>::default(),
        };

        let mut add_error = |span: Span, err: crate::Error| {
            error.combine(syn::Error::new(span, context.format(&err)));
        };
        let len = |id: &Id| source.get(id).map_or(0, Vec::len);
        let has = |id: &Id| len(id) > 0;
        let as_name = |id: Id| *names.get(&id).expect("undefined Id");

        // Update relationships from groups
        for (_, members) in group.iter() {
            for i in 0..members.len() {
                let this = members[i];
                for &that in members[0..i].iter().chain(members[i + 1..].iter()) {
                    conflicts_with.entry(this).or_default().push(that);
                }
            }
        }

        for id in required.iter() {
            if !has(id) {
                add_error(
                    context.node(),
                    Error::MissingRequirements {
                        missings: &[as_name(*id)],
                    },
                );
            }
        }

        for (id, action) in action.iter() {
            if let Some(spans) = source.get(id) {
                match action {
                    ArgAction::Set if spans.len() > 1 => {
                        let name = as_name(*id);
                        for span in spans {
                            add_error(*span, Error::DuplicateArg { name });
                        }
                    }
                    _ => {}
                }
            }
        }

        for (id, conflicts) in conflicts_with.iter() {
            if let Some(spans) = source.get(id) {
                debug_assert!(!spans.is_empty());
                let mut buffer = buffer.acquire();
                buffer.extend(conflicts.iter().copied().filter(has).map(as_name));
                if buffer.is_empty() {
                    continue;
                }
                let name = as_name(*id);
                for span in spans.iter() {
                    add_error(
                        *span,
                        Error::ConflictArgs {
                            name,
                            conflicts: &buffer,
                        },
                    );
                }
            }
        }

        for (id, requirements) in requires.iter() {
            if let Some(spans) = source.get(id) {
                let mut buffer = buffer.acquire();
                buffer.extend(requirements.iter().copied().filter(has).map(as_name));
                if buffer.is_empty() {
                    continue;
                }
                for span in spans.iter() {
                    add_error(*span, Error::MissingRequirements { missings: &buffer });
                }
            }
        }

        error.inner.take().map_or(Ok(()), Err)
    }
}

#[derive(Default)]
struct ErrorCollector {
    inner: Option<syn::Error>,
}

impl ErrorCollector {
    fn combine(&mut self, e: syn::Error) {
        if let Some(err) = self.inner.as_mut() {
            err.combine(e);
        } else {
            self.inner = Some(e);
        }
    }
}

struct Buffer<T> {
    inner: Vec<T>,
}

impl<T> Buffer<T> {
    pub fn acquire(&mut self) -> BufferGuard<T> {
        BufferGuard {
            inner: &mut self.inner,
        }
    }
}

struct BufferGuard<'a, T> {
    inner: &'a mut Vec<T>,
}

impl<'a, T> Extend<T> for BufferGuard<'a, T>
where
    T: Ord,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter);
        self.inner.sort();
    }
}

impl<T> ops::Deref for BufferGuard<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<T> ops::DerefMut for BufferGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Drop for BufferGuard<'_, T> {
    fn drop(&mut self) {
        self.inner.clear();
    }
}
