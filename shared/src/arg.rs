use crate::{
    runtime::{Id, Rt},
    Name,
};
use proc_macro2::Span;

/// An user-defined argument.
pub struct Arg<T> {
    id: Id,
    rt: Rt,
    values: Vec<(T, Span)>,
}

/// Behavior of arguments when they are encountered while parsing.
#[non_exhaustive]
pub enum ArgAction {
    /// Returns at most one value in [`Arg`].
    Set,
    /// Returns all associated values in [`Arg`].
    Append,
}

impl<T> Arg<T> {
    pub(crate) fn new(id: Id, rt: Rt) -> Self {
        Self {
            id,
            rt,
            values: <_>::default(),
        }
    }

    /// Specifies how to react to an argument when parsing it.
    pub fn action(self, action: ArgAction) -> Self {
        self.rt.borrow_mut().add_action(self.id, action);
        self
    }

    /// Specifies that the argument must be present and returns at least one value.
    pub fn required(self) -> Self {
        self.rt.borrow_mut().add_required(self.id);
        self
    }

    /// Sets an argument/group required when this one is present
    pub fn requires(self, name: Name) -> Self {
        self.rt.borrow_mut().add_requires(self.id, name);
        self
    }

    /// This argument is mutually exclusive with the specified argument/group.
    pub fn conflicts_with(self, name: Name) -> Self {
        self.rt.borrow_mut().add_conflicts_with(self.id, name);
        self
    }

    /// The name of the group which the argument belongs to. Every argument in
    /// the group conflicts with each other.
    pub fn group(self, name: Name) -> Self {
        self.rt.borrow_mut().add_to_group(self.id, name);
        self
    }

    /// Marks this argument as unexpected.
    pub fn set_unexpected(&mut self) {
        self.rt.borrow_mut().add_unexpected(self.id);
    }

    /// Collects a value for this argument.
    pub fn add_value(&mut self, span: Span, value: T) {
        self.values.push((value, span));
        self.rt.borrow_mut().add_source(self.id, span);
    }

    /// Returns the number if encountered values.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns `true` if no value encountered.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns all encountered values and spans.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            inner: self.values.iter(),
        }
    }

    /// Returns all encountered values.
    pub fn values(&self) -> Values<T> {
        Values {
            inner: self.values.iter(),
        }
    }

    /// Returns the spans of all encountered values.
    pub fn spans(&self) -> Spans<T> {
        Spans {
            inner: self.values.iter(),
        }
    }

    /// Returns the only optional value.
    pub fn option(&self) -> Option<&T> {
        assert!(self.values.len() <= 1);
        self.values().next()
    }

    /// Consumes itself and returns all encountered values.
    pub fn into_values(self) -> IntoValues<T> {
        IntoValues {
            inner: self.values.into_iter(),
        }
    }

    /// Consumes itself and returns the spans of all encountered values.
    pub fn into_spans(self) -> IntoSpans<T> {
        IntoSpans {
            inner: self.values.into_iter(),
        }
    }

    /// Consumes itself and collects all values into a [`Vec`].
    pub fn into_vec(self) -> Vec<T> {
        self.into_values().collect()
    }

    /// Consumes itself and returns the only optional value.
    pub fn into_option(self) -> Option<T> {
        assert!(self.values.len() <= 1);
        self.into_values().next()
    }
}

impl<T> IntoIterator for Arg<T> {
    type IntoIter = IntoIter<T>;
    type Item = (T, Span);

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.values.into_iter(),
        }
    }
}

macro_rules! define_iter {
    () => {};
    ($(#[$attr:meta])* $name:ident [$item:ty] => self.$inner:ident $(.$mem:tt)? ; $($rest:tt)*) => {
        $(#[$attr])*
        pub struct $name<T> {
            $inner: <Vec<(T, Span)> as IntoIterator>::IntoIter,
        }

        impl<T> Iterator for $name<T> {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.$inner.next() $(.map(|t| t.$mem))*
            }
        }

        define_iter!($($rest)*);
    };
    ($(#[$attr:meta])* & $name:ident [$item:ty] => self.$inner:ident $(.$mem:tt)? ; $($rest:tt)*) => {
        $(#[$attr])*
        pub struct $name<'a, T> {
            $inner: <&'a [(T, Span)] as IntoIterator>::IntoIter,
        }

        impl<'a, T> Iterator for $name<'a, T> {
            type Item = &'a $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.$inner.next() $(.map(|t| &t.$mem))*
            }
        }

        define_iter!($($rest)*);
    };
}
define_iter!(
    /// An iterator over the values and spans of an [`Arg`].
    IntoIter [(T, Span)] => self.inner;

    /// An iterator over the values of an [`Arg`].
    IntoValues [T] => self.inner.0;

    /// An iterator over the spans of an [`Arg`].
    IntoSpans [Span] => self.inner.1;

    /// An iterator over the values and spans of an [`Arg`].
    & Iter [(T, Span)] => self.inner;

    /// An iterator over the values of an [`Arg`].
    & Values [T] => self.inner.0;

    /// An iterator over the spans of an [`Arg`].
    & Spans [Span] => self.inner.1;
);

/// A logical group of related [`Arg`]s.
pub struct ArgGroup {
    id: Id,
    rt: Rt,
}

impl ArgGroup {
    pub(crate) fn new(id: Id, rt: Rt) -> Self {
        Self { id, rt }
    }

    /// Adds an argument to this group.
    pub fn arg(self, name: Name) -> Self {
        self.rt.borrow_mut().add_member(self.id, name);
        self
    }

    /// Specifies any of the arguments in this group must be present.
    pub fn required(self) -> Self {
        self.rt.borrow_mut().add_required(self.id);
        self
    }

    /// Requires the specified argument/group when any of the arguments in this
    /// group is present.
    pub fn requires(self, name: Name) -> Self {
        self.rt.borrow_mut().add_requires(self.id, name);
        self
    }

    /// Every argument in in this group is mutually exclusive with the specified
    /// argument/group.
    pub fn conflicts_with(self, name: Name) -> Self {
        self.rt.borrow_mut().add_conflicts_with(self.id, name);
        self
    }

    /// Marks every argument in this group as unexpected.
    pub fn set_unexpected(&mut self) {
        self.rt.borrow_mut().add_unexpected(self.id);
    }
}
