use plap::{Args, Checker, Parser};
use proc_macro2::Span;
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static GLOBAL_REGISTRY: RefCell<Registry> = RefCell::default();
}

/// A struct contains all included argument definitions.
#[derive(Default)]
struct Registry {
    defs: BTreeMap<&'static str, fn() -> Box<dyn DynArgs>>,
}

/// An object-safe version of [`Args`].
pub(crate) trait DynArgs {
    fn reset(&mut self);
    fn parse_next(&mut self, parser: &mut Parser) -> plap::Result<Span>;
    fn check_with(&self, checker: &mut Checker);
}

impl<T: Args> DynArgs for T {
    fn reset(&mut self) {
        *self = Args::init();
    }
    fn parse_next(&mut self, parser: &mut Parser) -> plap::Result<Span> {
        Args::parse_next(self, parser)
    }
    fn check_with(&self, checker: &mut Checker) {
        Args::check_with(self, checker)
    }
}

pub(crate) fn register<T: 'static + Args>(name: &'static str) {
    if GLOBAL_REGISTRY
        .with(|r| r.borrow_mut().defs.insert(name, || Box::new(T::init())))
        .is_some()
    {
        panic!("conflicting definition: {name}")
    }
}

pub(crate) fn get(name: &str) -> Option<Box<dyn DynArgs>> {
    GLOBAL_REGISTRY
        .with(|r| r.borrow().defs.get(name).copied())
        .map(|f| f())
}
