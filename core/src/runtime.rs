use crate::{
    arg::{Arg, ArgAction, ArgBuilder, ArgState},
    error::{DefaultFormatter, DelegatedFormatter, Error, Error::*, ErrorFormatter},
    group::{GroupBuilder, GroupState},
    Name, RawName, DUMMY_NAME,
};
use proc_macro2::Span;
use std::{
    collections::{btree_map, BTreeMap},
    fmt,
};
use syn::{Error as SynError, Result};

pub struct Runtime {
    node: Span,
    formatter: DelegatedFormatter,
    states: Vec<State>,
}

#[must_use]
pub struct RuntimeBuilder {
    node: Option<Span>,
    namespace: Option<RawName>,
    formatter: Option<Box<dyn ErrorFormatter>>,
    ids: BTreeMap<RawName, Id>,
    states: Vec<State>,
}

enum State {
    Undefined(Undefined),
    Arg(ArgState),
    Group(GroupState),
}

struct Undefined {
    name: RawName,
}

impl State {
    pub fn name_mut(&mut self) -> &mut RawName {
        match self {
            Self::Undefined(s) => &mut s.name,
            Self::Arg(s) => &mut s.name,
            Self::Group(s) => &mut s.name,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Id(usize);

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            node: None,
            namespace: None,
            formatter: None,
            ids: BTreeMap::new(),
            states: Vec::new(),
        }
    }

    pub fn capacity(mut self, capacity: usize) -> Self {
        self.states = Vec::with_capacity(capacity);
        self
    }

    pub fn node(mut self, node: Span) -> Self {
        self.node = Some(node);
        self
    }

    pub fn namespace<N>(mut self, namespace: N) -> Self
    where
        N: Into<Name>,
    {
        self.namespace = Some(namespace.into().0);
        self
    }

    pub fn formatter<F>(mut self, formatter: F) -> Self
    where
        F: 'static + ErrorFormatter,
    {
        self.formatter = Some(Box::new(formatter));
        self
    }

    pub(crate) fn register<N>(&mut self, name: N) -> Id
    where
        N: Into<Name>,
    {
        fn register_impl(rt: &mut RuntimeBuilder, name: Name) -> Id {
            debug_assert_eq!(rt.ids.len(), rt.states.len());
            let Name(name) = name;
            match rt.ids.entry(name.clone()) {
                btree_map::Entry::Occupied(t) => *t.get(),
                btree_map::Entry::Vacant(t) => {
                    let id = Id(rt.states.len());
                    rt.states.push(State::Undefined(Undefined { name }));
                    t.insert(id);
                    id
                }
            }
        }
        register_impl(self, name.into())
    }

    fn track_state(&mut self, id: Id, mut state: State) {
        debug_assert_eq!(state.name_mut(), &DUMMY_NAME);
        let slot = &mut self.states[id.0];
        match slot {
            State::Undefined(s) => {
                std::mem::swap(&mut s.name, state.name_mut());
                *slot = state;
            }
            _ => panic!("duplicate definition for `{}`", slot.name_mut()),
        }
    }

    pub fn arg<N, T>(&mut self, name: N) -> ArgBuilder<T>
    where
        N: Into<Name>,
    {
        ArgBuilder::new(self.register(name), self)
    }

    pub(crate) fn finish_arg(&mut self, id: Id, arg: ArgState) {
        self.track_state(id, State::Arg(arg));
    }

    pub fn group<N>(&mut self, name: N) -> GroupBuilder
    where
        N: Into<Name>,
    {
        GroupBuilder::new(self.register(name), self)
    }

    pub(crate) fn finish_group(&mut self, id: Id, grp: GroupState) {
        self.track_state(id, State::Group(grp));
    }

    pub fn finish(self) -> Runtime {
        let Self {
            node,
            namespace,
            formatter,
            ids: _,
            states,
        } = self;
        for state in states.iter() {
            if let State::Undefined(Undefined { name }) = state {
                panic!("missing definition for `{}`", name);
            }
        }
        Runtime {
            node: node.unwrap_or_else(|| Span::call_site()),
            states,
            formatter: formatter
                .map(DelegatedFormatter::Other)
                .unwrap_or_else(|| DelegatedFormatter::Default(DefaultFormatter { namespace })),
        }
    }
}

impl Runtime {
    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::new()
    }

    pub(crate) fn track_source(&mut self, id: Id, span: Span) {
        self.states
            .get_mut(id.0)
            .and_then(|s| if let State::Arg(s) = s { Some(s) } else { None })
            .expect("given argument does not belong to current runtime")
            .sources
            .push(span);
    }

    pub fn track_arg<T>(&mut self, arg: &mut Arg<T>, span: Span, val: T) {
        self.track_source(arg.id(), span);
        arg.add_value(val);
    }

    pub fn finish(self) -> Result<()> {
        self.check()
    }

    fn check(&self) -> Result<()> {
        RuntimeChecker {
            rt: self,
            supplied: vec![None; self.states.len()],
            buffer: Vec::new(),
            error: None,
        }
        .check()
    }

    fn state(&self, id: Id) -> &State {
        &self.states[id.0]
    }

    fn visit_args<'a>(&'a self, state: &'a State, mut f: impl FnMut(&'a ArgState)) {
        fn visit_args_impl<'a, F>(rt: &'a Runtime, state: &'a State, f: &mut F)
        where
            F: FnMut(&'a ArgState),
        {
            match state {
                State::Arg(arg) => f(arg),
                State::Group(grp) => {
                    // In most cases, a group is not nested, so that we can
                    // avoid recursive calls.
                    for id in grp.members.iter().copied() {
                        match rt.state(id) {
                            State::Arg(arg) => f(arg),
                            state => visit_args_impl(rt, state, f),
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        visit_args_impl(self, state, &mut f)
    }

    fn fmt_error(&self, err: &Error) -> String {
        DisplayError(&self.formatter, err).to_string()
    }
}

struct RuntimeChecker<'a> {
    rt: &'a Runtime,
    supplied: Vec<Option<bool>>,
    buffer: Vec<&'a str>,
    error: Option<SynError>,
}

impl<'a> RuntimeChecker<'a> {
    pub fn check(mut self) -> Result<()> {
        let rt = self.rt;
        for (id, state) in rt.states.iter().enumerate() {
            let id = Id(id);
            let requires: &[Id];
            let conflicts: &[Id];
            match state {
                State::Arg(arg) => {
                    // A required argument must be supplied.
                    if arg.required && !self.supplied(id) {
                        self.missing_argument(&[rt.node], id);
                    }
                    // Validate value count.
                    if matches!(arg.action, ArgAction::Set) && arg.sources.len() > 1 {
                        for node in arg.sources.iter().copied() {
                            self.error(node, DuplicateValue);
                        }
                    }
                    requires = &arg.requires;
                    conflicts = &arg.conflicts;
                }
                State::Group(grp) => {
                    // All members of a required group must be supplied.
                    if grp.required && !self.supplied(id) {
                        self.missing_argument(&[rt.node], id);
                    }
                    // Arguments in a single-member group conflict with each other.
                    if !grp.multiple {
                        for i in 0..grp.members.len() {
                            let id = grp.members[i];
                            for conflicting in grp.members[..i]
                                .iter()
                                .chain(grp.members[i + 1..].iter())
                                .copied()
                            {
                                if !self.supplied(conflicting) {
                                    continue;
                                }
                                rt.visit_args(rt.state(id), |arg| {
                                    self.conflicting_argument(&arg.sources, conflicting);
                                });
                            }
                        }
                    }
                    requires = &grp.requires;
                    conflicts = &grp.conflicts;
                }
                _ => unreachable!(),
            }
            // Check for missing requirements.
            for id in requires.iter().copied() {
                if self.supplied(id) {
                    continue;
                }
                rt.visit_args(state, |arg| {
                    self.missing_argument(&arg.sources, id);
                });
            }
            // Check for conflicting arguments/groups.
            for id in conflicts.iter().copied() {
                if !self.supplied(id) {
                    continue;
                }
                rt.visit_args(state, |arg| {
                    self.conflicting_argument(&arg.sources, id);
                });
            }
        }

        self.error.map(Err).unwrap_or(Ok(()))
    }

    fn supplied(&mut self, id: Id) -> bool {
        fn check_supplied(rt: &Runtime, id: Id) -> bool {
            match &rt.state(id) {
                State::Arg(arg) => !arg.sources.is_empty(),
                State::Group(grp) => grp.members.iter().all(|id| check_supplied(rt, *id)),
                _ => unreachable!(),
            }
        }
        *self.supplied[id.0].get_or_insert_with(|| check_supplied(self.rt, id))
    }

    fn collect_args<'b, T>(&'b mut self, id: Id, f: impl FnOnce(&'b [&'a str]) -> T) -> T {
        self.buffer.clear();
        self.rt
            .visit_args(self.rt.state(id), |arg| self.buffer.push(&arg.name));
        f(&self.buffer)
    }

    fn error(&mut self, node: Span, err: Error) {
        self.error_syn(SynError::new(node, self.rt.fmt_error(&err)));
    }

    fn error_syn(&mut self, err: SynError) {
        if let Some(error) = self.error.as_mut() {
            error.combine(err);
        } else {
            self.error = Some(err);
        }
    }

    fn missing_argument(&mut self, nodes: &[Span], id: Id) {
        let rt = self.rt;
        let msg = self.collect_args(id, |args| rt.fmt_error(&MissingArgument { args }));
        for node in nodes.iter().copied() {
            self.error_syn(SynError::new(node, &msg));
        }
    }

    fn conflicting_argument(&mut self, nodes: &[Span], id: Id) {
        let rt = self.rt;
        let msg = self.collect_args(id, |args| rt.fmt_error(&ConflictingArgument { args }));
        for node in nodes.iter().copied() {
            self.error_syn(SynError::new(node, &msg));
        }
    }
}

struct DisplayError<'a, F>(&'a F, &'a Error<'a>);

impl<F> fmt::Display for DisplayError<'_, F>
where
    F: ErrorFormatter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f, self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! runtime {
        ($rt:ident { $($name:ident $([$($members:tt)*])? $(.$method:ident $arg:tt)*;)* }) => {
            let mut $rt = Runtime::builder();
            $(#[allow(unused)]
            let mut $name =
                runtime!(@new $rt, stringify!($name), [$($($members)*)*])
                $(.$method $arg)*
                .finish();)*
            #[allow(unused)]
            let mut $rt = $rt.finish();

        };
        (@new $rt:ident, $name:expr, []) => {
            $rt.arg::<_, ()>($name)
        };
        (@new $rt:ident, $name:expr, [$($member:ident),*]) => {
            $rt.group($name) $(.arg(stringify!($member)))*
        };
    }

    macro_rules! track_arg {
        ($rt:ident, $arg:ident) => {
            $rt.track_arg(&mut $arg, Span::call_site(), ());
        };
    }

    #[test]
    #[should_panic = "missing definition for `arg2`"]
    fn panic_on_missing_definition() {
        runtime!(rt { arg1.requires("arg2"); });
    }

    #[test]
    #[should_panic = "duplicate definition for `arg2`"]
    fn panic_on_duplicate_definition() {
        runtime!(rt { arg1; arg2; arg2; });
    }

    #[test]
    #[should_panic = "given argument does not belong to current runtime"]
    fn panic_on_mismatched_runtime() {
        runtime!(rt1 { arg1; });
        runtime!(rt2 { arg2; arg3; });
        track_arg!(rt1, arg3);
    }

    #[test]
    fn check_arg_state() {
        // required

        runtime!(rt { arg1.required(); });
        assert!(rt.check().is_err());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // at most one value when action is 'set'

        runtime!(rt { arg1.action(ArgAction::Set); });
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_err());

        // any count of arguments when action is 'append'

        runtime!(rt { arg1.action(ArgAction::Append); });
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // requires an argument

        runtime!(rt { arg1; arg2.requires("arg1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_err());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // requires a group

        runtime!(rt { arg1; grp1[arg1]; arg2.requires("grp1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_err());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // conflicts with an argument

        runtime!(rt { arg1; arg2.conflicts("arg1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_err());

        // conflicts with a group

        runtime!(rt { arg1; grp1[arg1]; arg2.conflicts("grp1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_err());
    }

    #[test]
    fn check_group_state() {
        // required

        runtime!(rt { arg1; grp1[arg1].required(); });
        assert!(rt.check().is_err());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // single-member group

        runtime!(rt { arg1; arg2; grp1[arg1,arg2]; });
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg2);
        assert!(rt.check().is_err());

        // multiple-members group

        runtime!(rt { arg1; arg2; grp1[arg1,arg2].multiple(); });
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg2);
        assert!(rt.check().is_ok());

        // requires an argument

        runtime!(rt { arg1; arg2; grp1[arg2].requires("arg1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_err());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // requires a group

        runtime!(rt { arg1; arg2; grp1[arg1]; grp2[arg2].requires("grp1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_err());

        track_arg!(rt, arg1);
        assert!(rt.check().is_ok());

        // conflicts with an argument

        runtime!(rt { arg1; arg2; grp1[arg2].conflicts("arg1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_err());

        // conflicts with a group

        runtime!(rt { arg1; arg2; grp1[arg1]; grp2[arg2].conflicts("grp1"); });
        track_arg!(rt, arg2);
        assert!(rt.check().is_ok());

        track_arg!(rt, arg1);
        assert!(rt.check().is_err());
    }
}
