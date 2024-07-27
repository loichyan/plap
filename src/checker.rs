use std::fmt;

use proc_macro2::Span;

use crate::arg::*;
use crate::id::*;
use crate::parser::*;
use crate::schema::*;
use crate::util::{product, Array, Captures, Errors, FmtWith};

pub(crate) fn check(parser: &mut Parser) -> syn::Result<()> {
    let mut c = Checker {
        schema: parser.schema,
        values: &mut parser.values,
    };
    let errors = &mut parser.errors;

    // update state of each argument or group
    for i in 0..c.values.len() {
        c.update_state(0, i);
    }

    // print help and skip the rest checks,
    // states should be updated to avoid circular references
    if !parser.help_spans.is_empty() {
        errors.reset();
        let help = build_help(&c).expect("failed to build usage");
        for &span in parser.help_spans.iter() {
            errors.add_info(span, &help);
        }
        return errors.finish();
    }

    // check: exclusives
    for i in c
        .schema
        .exclusives
        .iter()
        .copied()
        .filter(|&i| c.provided_many(i))
    {
        match &c.value(i).kind {
            ValueKind::Arg(..) => c.emit_errors(errors, i, |_| "value is duplicated"),
            ValueKind::Group(_, g) => {
                // each member conflicts with others
                for (&i, &dest) in product(&g.members) {
                    c.emit_conflicts(errors, i, dest);
                }
            }
            ValueKind::None => unreachable!(),
        }
    }

    // check: required
    for i in c
        .schema
        .required
        .iter()
        .copied()
        .filter(|&i| !c.provided(i))
    {
        errors.add_msg(format!("`{}` is required", c.name(i)));
    }

    // check: requirements
    for &(i, ref dests) in c
        .schema
        .requirements
        .iter()
        .filter(|&&(i, _)| c.provided(i))
    {
        for dest in dests.iter().copied().filter(|&i| !c.provided(i)) {
            c.emit_errors(errors, i, |_| format!("requires `{}`", c.name(dest)));
        }
    }

    // check: conflicts
    for &(i, ref dests) in c.schema.conflicts.iter().filter(|&&(i, _)| c.provided(i)) {
        for dest in dests.iter().copied().filter(|&i| c.provided(i)) {
            c.emit_conflicts(errors, i, dest);
        }
    }

    // check: unacceptables
    for &(i, ref msg) in parser.unacceptables.iter().filter(|&&(i, _)| c.provided(i)) {
        c.emit_errors(errors, i, |_| msg);
    }

    errors.finish()
}

fn build_help(c: &Checker) -> Result<String, std::fmt::Error> {
    struct Help<'a> {
        id: &'a Id,
        help: Option<&'a str>, // none on groups
        required: bool,
        multiple: bool,
        conflicts_with: Vec<Idx>,
    }

    let add_conflicts = |helps: &mut [Help], i: Idx, dest: Idx| {
        c.visit(i, |i, _| {
            c.visit(dest, |dest, _| {
                helps[i].conflicts_with.push(dest);
                helps[dest].conflicts_with.push(i);
            });
        });
    };

    let mut helps = c
        .schema
        .infos()
        .iter()
        .map(|inf| {
            let help = if let InfoKind::Arg(a) = &inf.kind {
                Some(&*a.help)
            } else {
                None
            };
            Help {
                id: &inf.id,
                help,
                required: false,
                multiple: true,
                conflicts_with: <_>::default(),
            }
        })
        .collect::<Array<_>>();

    for &i in c.schema.required.iter() {
        helps[i].required = true;
    }

    for &i in c.schema.exclusives.iter() {
        match c.value(i).kind {
            ValueKind::Arg(..) => helps[i].multiple = false,
            ValueKind::Group(_, g) => {
                for (&i, &dest) in product(&g.members) {
                    add_conflicts(&mut helps, i, dest);
                }
            }
            ValueKind::None => unreachable!(),
        }
    }

    for &(i, ref dests) in c.schema.conflicts.iter() {
        for &dest in dests.iter() {
            c.visit(i, |i, _| {
                c.visit(dest, |dest, _| {
                    add_conflicts(&mut helps, i, dest);
                });
            });
        }
    }

    // arg1:
    //     Argument #1
    //
    //     Required:       true
    //     Multiple:      false
    //     Conflicts with: arg2, arg3
    //
    // ...
    use fmt::Write;
    let mut rendered = String::from("USAGE:\n\n");
    for h in helps.iter() {
        let help = if let Some(h) = h.help { h } else { continue };
        let f = &mut rendered;
        {
            writeln!(f, "{}:", h.id)?;
            writeln!(f, "    {}", help)?;
            writeln!(f, "    Required:       {}", h.required)?;
            writeln!(f, "    Multiple:       {}", h.multiple)?;
        }
        if !h.conflicts_with.is_empty() {
            writeln!(f, "    Conflicts with: {}", c.join_ids(&h.conflicts_with))?;
        }
        f.push('\n');
    }
    rendered.pop(); // remove trailing newlines

    Ok(rendered)
}

pub(crate) struct Checker<'a, 'b> {
    pub schema: &'a Schema,
    pub values: &'b mut [Value<'a>],
}

impl<'a, 'b> Checker<'a, 'b> {
    fn provided(&self, i: Idx) -> bool {
        self.value(i).state.provided()
    }

    fn provided_many(&self, i: Idx) -> bool {
        self.value(i).state == ValueState::ProvidedMany
    }

    fn value(&self, i: Idx) -> &Value<'a> {
        &self.values[i]
    }

    fn id(&self, i: Idx) -> &Id {
        self.schema.id(i)
    }

    fn name(&self, i: Idx) -> impl '_ + fmt::Display + Captures<'a> + Captures<'b> {
        use fmt::Display;
        FmtWith(move |f| {
            match self.value(i).kind {
                // fast path for a single argument
                ValueKind::Arg(..) => self.id(i).fmt(f),
                ValueKind::Group(..) => {
                    let mut first = true;
                    self.try_visit(i, |i, _| {
                        if !first {
                            f.write_str(" | ")?;
                        } else {
                            first = false;
                        }
                        self.id(i).fmt(f)?;
                        Ok(())
                    })?;
                    Ok(())
                }
                ValueKind::None => unreachable!(),
            }
        })
    }

    fn join_ids<'i>(
        &'i self,
        iter: &'i [Idx],
    ) -> impl 'i + fmt::Display + Captures<'a> + Captures<'b> {
        use fmt::Display;
        FmtWith(|f| {
            let mut iter = iter.iter().copied();
            let first = if let Some(t) = iter.next() {
                t
            } else {
                return Ok(());
            };
            self.id(first).fmt(f)?;
            for i in iter {
                f.write_str(", ")?;
                self.id(i).fmt(f)?;
            }
            Ok(())
        })
    }

    fn emit_conflicts(&self, errors: &mut Errors, i: Idx, dest: Idx) {
        self.visit_span(i, |i, i_span| {
            self.visit_span(dest, |dest, dest_span| {
                // conflicts are always bidirectional
                errors.add(syn_error!(i_span, "conflicts with `{}`", self.id(dest)));
                errors.add(syn_error!(dest_span, "conflicts with `{}`", self.id(i)));
            });
        });
    }

    fn emit_errors<S>(&self, errors: &mut Errors, i: Idx, mut e: impl FnMut(Idx) -> S)
    where
        S: fmt::Display,
    {
        self.visit_span(i, |i, span| errors.add(syn_error!(span, e(i))));
    }

    fn visit_span(&self, i: Idx, mut f: impl FnMut(Idx, Span)) {
        self.visit(i, |i, v| v.keys().iter().for_each(|s| f(i, s.span())));
    }

    fn visit(&self, i: Idx, mut f: impl FnMut(Idx, &dyn AnyArg)) {
        let _ = self.try_visit(i, move |i, a| {
            f(i, a);
            Ok::<_, std::convert::Infallible>(())
        });
    }

    fn try_visit<E>(
        &self,
        i: Idx,
        mut f: impl FnMut(Idx, &dyn AnyArg) -> Result<(), E>,
    ) -> Result<(), E> {
        self._try_visit(i, &mut f)
    }

    fn _try_visit<E>(
        &self,
        i: Idx,
        f: &mut dyn FnMut(Idx, &dyn AnyArg) -> Result<(), E>,
    ) -> Result<(), E> {
        match self.values[i].kind {
            ValueKind::Arg(ref a, _) => f(i, *a),
            ValueKind::Group(_, g) => {
                for &member in g.members.iter() {
                    self._try_visit(member, f)?;
                }
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn update_state(&mut self, prev: Idx, i: Idx) -> ValueState {
        let val = &mut self.values[i];
        match val.state {
            ValueState::None => match val.kind {
                ValueKind::None => {
                    panic!("`{}` is not added", self.id(i));
                }
                ValueKind::Arg(ref a, _) => {
                    val.state = ValueState::from_n(a.keys().len());
                    val.state
                }
                ValueKind::Group(_, g) => {
                    val.state = ValueState::Busy;
                    // g is copied, so that we can pass the borrow check
                    let _ = val;
                    let mut n = 0;
                    for &member in g.members.iter() {
                        if self.update_state(i, member).provided() {
                            n += 1;
                            // continue check to detect circular references and
                            // count all provided arguments
                        }
                    }
                    let val = &mut self.values[i];
                    if let ValueKind::Group(g, _) = &mut val.kind {
                        g.n = n;
                        val.state = ValueState::from_n(n);
                        val.state
                    } else {
                        unreachable!()
                    }
                }
            },
            ValueState::Busy => {
                panic!(
                    "found circular groups: `{}` and `{}`",
                    self.id(i),
                    self.id(prev)
                );
            }
            state => state,
        }
    }
}

impl ValueState {
    fn from_n(n: usize) -> Self {
        match n {
            0 => Self::Empty,
            1 => Self::Provided,
            _ => Self::ProvidedMany,
        }
    }

    fn provided(&self) -> bool {
        *self as u8 >= Self::Provided as u8
    }
}
