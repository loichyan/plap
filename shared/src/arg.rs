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
    /// Returns at most one value in [`Arg::finish`].
    Set,
    /// Returns all associated values in [`Arg::finish`].
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

    /// Specifies that the argument must be present. Returns at least one value
    /// in [`finish`].
    ///
    /// [`finish`]: Self::finish
    pub fn required(self) -> Self {
        self.rt.borrow_mut().add_required(self.id);
        self
    }

    /// This argument is mutually exclusive with the specified argument.
    pub fn conflicts_with(self, name: Name) -> Self {
        self.rt.borrow_mut().add_conflicts_with(self.id, name);
        self
    }

    /// Sets an argument that is required when this one is present
    pub fn requires(self, name: Name) -> Self {
        self.rt.borrow_mut().add_requires(self.id, name);
        self
    }

    /// The name of the group which the argument belongs to. Arguments in the group
    /// conflicts with each other.
    pub fn group(self, name: Name) -> Self {
        self.rt.borrow_mut().add_group(self.id, name);
        self
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

macro_rules! define_into_iter {
    ($name:ident [$item:ty] => self.$inner:ident $($next:tt)*) => {
        pub struct $name<T> {
            $inner: <Vec<(T, Span)> as IntoIterator>::IntoIter,
        }

        impl<T> Iterator for $name<T> {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.$inner $($next)*
            }
        }

    };
}
macro_rules! define_iter {
    ($name:ident [$item:ty] => self.$inner:ident $($next:tt)*) => {
        pub struct $name<'a, T> {
            $inner: <&'a [(T, Span)] as IntoIterator>::IntoIter,
        }

        impl<'a, T> Iterator for $name<'a, T> {
            type Item = &'a $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.$inner $($next)*
            }
        }

    };
}

define_into_iter!(IntoIter [(T, Span)] => self.inner.next());
define_into_iter!(IntoValues [T] => self.inner.next().map(|t| t.0));
define_into_iter!(IntoSpans [Span] => self.inner.next().map(|t| t.1));
define_iter!(Iter [(T, Span)] => self.inner.next());
define_iter!(Values [T] => self.inner.next().map(|t| &t.0));
define_iter!(Spans [Span] => self.inner.next().map(|t| &t.1));
