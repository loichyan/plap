use std::ops;

#[derive(Default)]
pub struct Errors(Option<syn::Error>);

impl Errors {
    pub fn combine(&mut self, e: syn::Error) {
        if let Some(err) = self.0.as_mut() {
            err.combine(e);
        } else {
            self.0 = Some(e);
        }
    }

    pub fn take(&mut self) -> Option<syn::Error> {
        self.0.take()
    }

    pub fn fail(self) -> syn::Result<()> {
        match self.0 {
            Some(e) => Err(e),
            None => Ok(()),
        }
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
