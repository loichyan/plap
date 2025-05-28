use proc_macro2::{Ident, Span};
use std::fmt;

#[non_exhaustive]
pub enum Error {
    BadSyntax(syn::Error),
    UnknownArgument(Ident),
}

impl Error {
    pub fn into_syn(self) -> syn::Error {
        match self {
            Self::BadSyntax(e) => e,
            Self::UnknownArgument(i) => syn::Error::new(i.span(), "unknown argument"),
        }
    }
}

impl From<syn::Error> for Error {
    fn from(value: syn::Error) -> Self {
        Error::BadSyntax(value)
    }
}

impl From<Error> for syn::Error {
    fn from(value: Error) -> Self {
        value.into_syn()
    }
}

#[derive(Debug, Default)]
pub struct Errors {
    e: Option<syn::Error>,
}

impl Errors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, err: syn::Error) {
        if let Some(e) = &mut self.e {
            e.combine(err);
        } else {
            self.e = Some(err);
        }
    }

    pub fn add_result<T>(&mut self, res: syn::Result<T>) -> Option<T> {
        match res {
            Ok(t) => Some(t),
            Err(e) => {
                self.add(e);
                None
            },
        }
    }

    pub fn add_at(&mut self, span: Span, msg: impl fmt::Display) {
        self.add(syn::Error::new(span, msg))
    }

    // TODO: fail() -> Result<()>
    pub fn fail<T>(&mut self) -> syn::Result<T>
    where
        T: Default,
    {
        match self.e.take() {
            Some(e) => Err(e),
            None => Ok(T::default()),
        }
    }
}
