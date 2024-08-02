use std::fmt;

use proc_macro2::{Ident, Span};

use crate::errors::Errors;

#[cfg_attr(docsrs, doc(cfg(feature = "checking")))]
pub trait AnyArg {
    fn name(&self) -> &str;

    fn keys(&self) -> &[Ident];
}

impl<T> AnyArg for crate::arg::Arg<T> {
    fn name(&self) -> &str {
        self.name()
    }

    fn keys(&self) -> &[Ident] {
        self.keys()
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "checking")))]
#[derive(Default)]
pub struct Checker {
    errors: Errors,
    spans: Vec<Span>,
}

impl Checker {
    pub fn with_result(&mut self, res: syn::Result<()>) -> &mut Self {
        self.errors.add_result(res);
        self
    }

    pub fn with_error(&mut self, err: syn::Error) -> &mut Self {
        self.errors.add(err);
        self
    }

    pub fn with_error_at(&mut self, span: Span, msg: impl fmt::Display) -> &mut Self {
        self.errors.add_at(span, msg);
        self
    }

    pub fn with_source(&mut self, span: Span) -> &mut Self {
        self.spans.push(span);
        self
    }

    pub fn with_error_at_source(&mut self, msg: impl fmt::Display + Clone) -> &mut Self {
        if self.spans.is_empty() {
            self.errors.add_at(Span::call_site(), msg);
        } else {
            for &span in self.spans.iter() {
                self.errors.add_at(span, msg.clone());
            }
        }
        self
    }

    /* ---------------------- *
     * container level checks *
     * ---------------------- */

    pub fn required_all<'a>(&mut self, args: impl AsRef<[&'a dyn AnyArg]>) -> &mut Self {
        self._required_all(args.as_ref())
    }

    fn _required_all(&mut self, args: &[&dyn AnyArg]) -> &mut Self {
        for &a in args {
            self.required(a);
        }
        self
    }

    pub fn required_any<'a>(&mut self, args: impl AsRef<[&'a dyn AnyArg]>) -> &mut Self {
        self._required_any(args.as_ref())
    }

    fn _required_any(&mut self, args: &[&dyn AnyArg]) -> &mut Self {
        if count_group(args) == 0 {
            self.with_error_at_source(format!("`{}` is required", fmt_group(args)));
        }
        self
    }

    pub fn exclusive_group<'a>(&mut self, args: impl AsRef<[&'a dyn AnyArg]>) -> &mut Self {
        self._exclusive_group(args.as_ref())
    }

    fn _exclusive_group(&mut self, args: &[&dyn AnyArg]) -> &mut Self {
        for (&a, &b) in combination(args) {
            self.conflicts_with(a, b);
        }
        self
    }

    pub fn exclusive_aliases<'a>(&mut self, args: impl AsRef<[&'a dyn AnyArg]>) -> &mut Self {
        self._exclusive_aliases(args.as_ref())
    }

    fn _exclusive_aliases(&mut self, args: &[&dyn AnyArg]) -> &mut Self {
        if count_group(args) > 1 {
            for &arg in args {
                self._too_many_values(arg);
            }
        }
        self
    }

    pub fn unallowed_all<'a>(&mut self, args: impl AsRef<[&'a dyn AnyArg]>) -> &mut Self {
        self._unallowed_all(args.as_ref())
    }

    fn _unallowed_all(&mut self, args: &[&dyn AnyArg]) -> &mut Self {
        for &a in args {
            self.unallowed(a);
        }
        self
    }

    /* ------------------ *
     * field level checks *
     * ------------------ */

    pub fn required(&mut self, arg: &dyn AnyArg) -> &mut Self {
        if arg.keys().is_empty() {
            self.with_error_at_source(format!("`{}` is required", arg.name()));
        }
        self
    }

    pub fn exclusive(&mut self, a: &dyn AnyArg) -> &mut Self {
        if a.keys().len() > 1 {
            self._too_many_values(a);
        }
        self
    }

    fn _too_many_values(&mut self, a: &dyn AnyArg) {
        for k in a.keys() {
            self.with_error_at(k.span(), "too many values (<= 1)");
        }
    }

    pub fn requires(&mut self, a: &dyn AnyArg, b: &dyn AnyArg) -> &mut Self {
        if b.keys().is_empty() {
            let b_name = b.name();
            for k in a.keys() {
                self.with_error_at(k.span(), format!("requires `{}`", b_name));
            }
        }
        self
    }

    pub fn requires_all<'b>(
        &mut self,
        a: &dyn AnyArg,
        b: impl AsRef<[&'b dyn AnyArg]>,
    ) -> &mut Self {
        self._requires_all(a, b.as_ref())
    }

    fn _requires_all(&mut self, a: &dyn AnyArg, b: &[&dyn AnyArg]) -> &mut Self {
        for &b in b {
            self.requires(a, b);
        }
        self
    }

    pub fn requires_any<'b>(
        &mut self,
        a: &dyn AnyArg,
        b: impl AsRef<[&'b dyn AnyArg]>,
    ) -> &mut Self {
        self._requires_any(a, b.as_ref())
    }

    fn _requires_any(&mut self, a: &dyn AnyArg, args: &[&dyn AnyArg]) -> &mut Self {
        if count_group(args) == 0 {
            for k in a.keys() {
                self.with_error_at(k.span(), format!("requires `{}`", fmt_group(args)));
            }
        }
        self
    }

    pub fn conflicts_with(&mut self, a: &dyn AnyArg, b: &dyn AnyArg) -> &mut Self {
        let b_keys = b.keys();
        for a in a.keys() {
            for b in b_keys {
                // conflicts are always bidirectional
                self.with_error_at(a.span(), format!("conflicts with `{}`", b));
                self.with_error_at(b.span(), format!("conflicts with `{}`", a));
            }
        }
        self
    }

    pub fn conflicts_with_any<'b>(
        &mut self,
        a: &dyn AnyArg,
        b: impl AsRef<[&'b dyn AnyArg]>,
    ) -> &mut Self {
        self._conflicts_with_any(a, b.as_ref())
    }

    fn _conflicts_with_any(&mut self, a: &dyn AnyArg, b: &[&dyn AnyArg]) -> &mut Self {
        for &b in b {
            self.conflicts_with(a, b);
        }
        self
    }

    pub fn unallowed(&mut self, a: &dyn AnyArg) -> &mut Self {
        for k in a.keys() {
            self.with_error_at(k.span(), "not allowed in this context");
        }
        self
    }

    pub fn finish(&mut self) -> syn::Result<()> {
        self.spans.clear();
        self.errors.fail()
    }
}

fn count_group(args: &[&dyn AnyArg]) -> usize {
    args.iter().map(|a| a.keys().len()).sum()
}

fn combination<T>(arr: &[T]) -> impl '_ + Iterator<Item = (&'_ T, &'_ T)> {
    arr.iter()
        .enumerate()
        .flat_map(|(k, t1)| arr[(k + 1)..].iter().map(move |t2| (t1, t2)))
}

fn fmt_group<'a>(args: &'a [&dyn AnyArg]) -> impl 'a + fmt::Display {
    FmtWith(|f| {
        use fmt::Display;
        let mut iter = args.iter();
        if let Some(first) = iter.next() {
            first.name().fmt(f)?;
        }
        for a in iter {
            f.write_str(" | ")?;
            a.name().fmt(f)?;
        }
        Ok(())
    })
}

struct FmtWith<F>(pub F)
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
