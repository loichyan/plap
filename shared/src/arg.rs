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

    /// Returns all encountered values.
    ///
    /// **Note:** This function should be called after [`ParserContext::finish`]
    /// to ensure the number of returned value(s) is correct.
    ///
    /// [`ParserContext::finish`]: crate::ParserContext::finish
    pub fn finish(self) -> Vec<(T, Span)> {
        self.values
    }
}

/// Behavior of arguments when they are encountered while parsing.
#[non_exhaustive]
pub enum ArgAction {
    /// Returns at most one value in [`Arg::finish`].
    Set,
    /// Returns all associated values in [`Arg::finish`].
    Append,
}
