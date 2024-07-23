use std::{fmt, ops};

use proc_macro2::Span;

macro_rules! is_debug {
    () => {
        cfg!(debug_assertions)
    };
}

macro_rules! syn_error {
    ($span:expr, $fmt:literal, $($args:tt)+) => {
        ::syn::Error::new($span, format!($fmt, $($args)*))
    };
    ($span:expr, $msg:expr $(,)?) => {
        ::syn::Error::new($span, $msg)
    };
}

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
        // TODO: how can we emit warnings/infos intead of errors?
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

    pub fn take(&mut self) -> Option<syn::Error> {
        self.e.take()
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

pub struct Buffer<T> {
    inner: Vec<T>,
}

impl<T> Default for Buffer<T> {
    fn default() -> Self {
        Self {
            inner: <_>::default(),
        }
    }
}

impl<T> Buffer<T> {
    pub fn acquire<I>(&mut self, iter: I) -> BufferGuard<T>
    where
        T: Ord,
        I: IntoIterator<Item = T>,
    {
        self.inner.extend(iter);
        self.inner.sort();
        BufferGuard {
            inner: &mut self.inner,
        }
    }
}

pub struct BufferGuard<'a, T> {
    inner: &'a mut Vec<T>,
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

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Iterator for Either<L, R>
where
    L: Iterator,
    R: Iterator<Item = L::Item>,
{
    type Item = L::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(l) => l.next(),
            Self::Right(r) => r.next(),
        }
    }
}
