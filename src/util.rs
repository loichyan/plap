use std::fmt;

use proc_macro2::Span;

pub trait Captures<'__> {}

impl<T: ?Sized> Captures<'_> for T {}

pub struct Errors {
    e: Option<syn::Error>,
    span: Span,
}

impl Errors {
    pub fn set_span(&mut self, span: Span) {
        self.span = span;
    }

    pub fn add_info(&mut self, span: Span, msg: impl std::fmt::Display) {
        // TODO: how can we emit warnings/infos instead of errors?
        self.add(syn_error!(span, msg));
    }

    pub fn add_msg(&mut self, msg: impl fmt::Display) {
        self.add(syn::Error::new(self.span, msg))
    }

    pub fn add(&mut self, e: syn::Error) {
        if let Some(err) = self.e.as_mut() {
            err.combine(e);
        } else {
            self.e = Some(e);
        }
    }

    pub fn fail(self) -> syn::Result<()> {
        match self.e {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Default for Errors {
    fn default() -> Self {
        Self {
            e: <_>::default(),
            span: Span::call_site(),
        }
    }
}

pub struct FmtWith<F>(pub F)
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result;

impl<F> fmt::Display for FmtWith<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}

impl<F> fmt::Debug for FmtWith<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f)
    }
}
