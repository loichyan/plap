use std::fmt;

use proc_macro2::Span;

#[derive(Debug, Default)]
pub struct Errors {
    e: Option<syn::Error>,
}

impl Errors {
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
            }
        }
    }

    pub fn add_at(&mut self, span: Span, msg: impl fmt::Display) {
        self.add(syn::Error::new(span, msg))
    }

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
