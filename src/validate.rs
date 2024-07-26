use std::fmt;

use crate::arg::*;
use crate::id::*;
use crate::parser::*;
use crate::schema::*;
use crate::util::{Captures, Errors, FmtWith};

pub(crate) fn validate(parser: &mut Parser) -> syn::Result<()> {
    let mut c = Checker {
        schema: parser.schema,
        values: &mut parser.values,
    };
    let errors = &mut parser.errors;

    // check if each argument or group is provided
    for i in 0..c.values.len() {
        c.update_state(0, i);
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
            ValueKind::Arg(..) => c.emit_errors(errors, i, || "value is duplicated"),
            ValueKind::Group(_, g) => {
                // each member conflicts with others
                for (k, &i) in g.members.iter().enumerate() {
                    c.visit(i, |_, arg| {
                        for &span in arg.spans() {
                            for &j in g.members[0..k].iter().chain(g.members[(k + 1)..].iter()) {
                                errors.add(syn_error!(span, "conflicts with `{}`", c.name(j)));
                            }
                        }
                    });
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
    for &(i, ref requirements) in c
        .schema
        .requirements
        .iter()
        .filter(|&&(i, _)| c.provided(i))
    {
        for dest in requirements.iter().copied().filter(|&i| !c.provided(i)) {
            c.emit_errors(errors, i, || format!("requires `{}`", c.name(dest)));
        }
    }

    // check: conflicts
    for &(i, ref conflicts) in c.schema.conflicts.iter().filter(|&&(i, _)| c.provided(i)) {
        for dest in conflicts.iter().copied().filter(|&i| c.provided(i)) {
            // conflicts are always bidirectional
            c.emit_errors(errors, i, || format!("conflicts with `{}`", c.name(dest)));
            c.emit_errors(errors, dest, || format!("conflicts with `{}`", c.name(i)));
        }
    }

    // check: unacceptables
    for &(i, ref msg) in parser.unacceptables.iter().filter(|&&(i, _)| c.provided(i)) {
        c.emit_errors(errors, i, || msg);
    }

    parser.errors.finish()
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
                    let mut last = None;
                    self.try_visit(i, |i, _| {
                        if let Some(i) = last.replace(i) {
                            self.id(i).fmt(f)?;
                            f.write_str(" | ")?;
                        }
                        Ok(())
                    })?;
                    if let Some(i) = last {
                        self.id(i).fmt(f)?;
                    }
                    Ok(())
                }
                ValueKind::None => unreachable!(),
            }
        })
    }

    fn emit_errors<S>(&self, errors: &mut Errors, i: Idx, mut e: impl FnMut() -> S)
    where
        S: fmt::Display,
    {
        self.visit(i, |_, arg| {
            for &span in arg.spans() {
                errors.add(syn_error!(span, e()));
            }
        })
    }

    fn visit(&self, i: Idx, mut f: impl FnMut(Idx, &dyn AnyArg)) {
        self.try_visit(i, move |i, a| {
            f(i, a);
            Ok::<_, std::convert::Infallible>(())
        })
        .ok();
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
                    val.state = ValueState::from_n(a.spans().len());
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
                            // continue check to detect circular reference and
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
