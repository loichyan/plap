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

    pub fn add_action(&mut self, this: Id, action: ArgAction) {
        self.action.insert(this, action);
    }

    pub fn add_required(&mut self, this: Id) {
        self.required.insert(this);
    }

    pub fn add_conflicts_with(&mut self, this: Id, that: Name) {
        let that = self.register(that);
        self.conflicts_with.entry(this).or_default().push(that);
        self.conflicts_with.entry(that).or_default().push(this);
    }

    pub fn add_requires(&mut self, this: Id, that: Name) {
        let that = self.register(that);
        self.requires.entry(this).or_default().push(that);
    }

    pub fn add_source(&mut self, this: Id, span: Span) {
        self.source.entry(this).or_default().push(span);
    }

    pub fn add_error(&mut self, span: Span, msg: String) {
        self.add_syn_error(syn::Error::new(span, msg));
    }

    pub fn add_syn_error(&mut self, e: syn::Error) {
        self.error.combine(e);
    }

    pub fn finish(&mut self, context: &ParserContext) -> Result<()> {
        use crate::Error;

        let Self {
            names,
            source,
            error,
            action,
            required,
            conflicts_with,
            requires,
            ..
        } = self;
        let mut buffer = Buffer::<&str> {
            inner: <_>::default(),
        };

        let mut add_error = |span: Span, err: crate::Error| {
            error.combine(syn::Error::new(span, context.format(&err)));
        };
        let len = |id: &Id| source.get(id).map(Vec::len).unwrap_or(0);
        let has = |id: &Id| len(id) > 0;
        let as_name = |id: Id| *names.get(&id).expect("undefined Id");

        for this in required.iter() {
            if !has(this) {
                add_error(
                    context.node(),
                    Error::MissingRequirements {
                        those: &[as_name(*this)],
                    },
                );
            }
        }

        for (this, action) in action.iter() {
            if let Some(spans) = source.get(this) {
                match action {
                    ArgAction::Set if spans.len() > 1 => {
                        let this = as_name(*this);
                        for span in spans {
                            add_error(*span, Error::DuplicateArg { this });
                        }
                    }
                    _ => {}
                }
            }
        }

        for (this, conflicts) in conflicts_with.iter() {
            if let Some(spans) = source.get(this) {
                debug_assert!(!spans.is_empty());
                let mut buffer = buffer.acquire();
                buffer.extend(conflicts.iter().copied().filter(has).map(as_name));
                if buffer.is_empty() {
                    continue;
                }
                let this = as_name(*this);
                for span in spans.iter() {
                    add_error(
                        *span,
                        Error::ConflictArgs {
                            this,
                            those: &buffer,
                        },
                    );
                }
            }
        }

        for (this, requirements) in requires.iter() {
            if let Some(spans) = source.get(this) {
                let mut buffer = buffer.acquire();
                buffer.extend(requirements.iter().copied().filter(has).map(as_name));
                if buffer.is_empty() {
                    continue;
                }
                for span in spans.iter() {
                    add_error(*span, Error::MissingRequirements { those: &buffer });
                }
            }
        }

        error.inner.take().map(Err).unwrap_or(Ok(()))
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
