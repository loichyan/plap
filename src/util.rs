use std::fmt;

use proc_macro2::Span;

pub(crate) type Array<T> = Box<[T]>;

pub(crate) trait Captures<'__> {}

impl<T: ?Sized> Captures<'_> for T {}

#[derive(Debug, Default)]
pub(crate) struct Errors {
    e: Option<syn::Error>,
    spans: Vec<Span>,
}

impl Errors {
    pub fn add_span(&mut self, span: Span) {
        self.spans.push(span);
    }

    pub fn add_info(&mut self, span: Span, msg: impl fmt::Display) {
        // TODO: how can we emit warnings/infos instead of errors?
        self.add(syn_error!(span, msg));
    }

    pub fn add_msg(&mut self, msg: impl fmt::Display + Clone) {
        if self.spans.is_empty() {
            self.add(syn::Error::new(Span::call_site(), msg));
        } else {
            for &span in self.spans.iter() {
                add_err(&mut self.e, syn::Error::new(span, msg.clone()));
            }
        }
    }

    pub fn add(&mut self, e: syn::Error) {
        add_err(&mut self.e, e);
    }

    pub fn finish(&mut self) -> syn::Result<()> {
        self.spans.clear();
        match self.e.take() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub fn reset(&mut self) {
        self.e.take();
        self.spans.clear();
    }
}

fn add_err(target: &mut Option<syn::Error>, e: syn::Error) {
    if let Some(target) = target {
        target.combine(e);
    } else {
        *target = Some(e);
    }
}

pub(crate) struct FmtWith<F>(pub F)
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

pub(crate) fn product<'a, T>(arr: &'a [T]) -> impl '_ + Iterator<Item = (&'a T, &'a T)> {
    arr.iter()
        .enumerate()
        .flat_map(|(k, t1)| arr[(k + 1)..].iter().map(move |t2| (t1, t2)))
}
