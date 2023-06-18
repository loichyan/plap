use crate::{ast, runtime::Rt, Arg, DefaultFormatter, ErrorFormatter};
use proc_macro2::Span;
use syn::{parse::ParseStream, Ident, Result, Token};

/// Parse input stream into user-defined container.
pub trait Parser: Sized {
    type Output;

    /// Returns the context of current parser.
    fn context(&self) -> &ParserContext;

    /// Attempts to parse an encountered argument and returns `false` if the input
    /// stream cannot be parsed.
    fn parse_once(&mut self, input: ParseStream) -> Result<bool>;

    /// Parses the input stream as comma-separated arguments.
    fn parse(&mut self, input: ParseStream) -> Result<()> {
        loop {
            if input.is_empty() {
                break;
            }

            // Report unknown arguments.
            if !self.parse_once(input)? {
                let span = input.span();
                let context = self.context();
                let mut rt = context.rt.borrow_mut();
                let err = if input.peek(Ident) {
                    let ident = input.parse::<Ident>()?;
                    context.format(&crate::Error::UnknownArg {
                        this: &ident.to_string(),
                    })
                } else {
                    context.format(&crate::Error::InvalidInput)
                };
                ast::parse_util_comma(input)?;
                rt.add_error(span, err);
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(())
    }

    /// Completes parsing and returns errors that occurred during parsing.
    ///
    /// **Note:** This function should combine all encountered validation errors into
    /// a single error.
    fn finish(self) -> Result<Self::Output>;
}

/// The runtime context for a [`Parser`].
pub struct ParserContext {
    node: Span,
    args: Vec<&'static str>,
    formatter: Box<dyn ErrorFormatter>,
    rt: Rt,
}

/// An identifier for [`Arg`].
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Id(usize);

impl ParserContext {
    pub fn new(node: Span) -> Self {
        Self::builder().node(node).build()
    }

    /// Overrides the default contexts.
    pub fn builder() -> ParserContextBuilder {
        ParserContextBuilder::default()
    }

    /// Returns the current [`node`].
    ///
    /// [`node`]: ParserContextBuilder::node
    pub fn node(&self) -> Span {
        self.node
    }

    /// Displays an [`Error`] using configured [`formatter`].
    ///
    /// [`Error`]: crate::Error
    /// [`formatter`]: ParserContextBuilder::formatter
    pub fn format(&self, err: &crate::Error) -> String {
        self.formatter.fmt(err)
    }

    /// Returns the name of a registered argument.
    pub fn name(&self, id: Id) -> &str {
        self.args.get(id.0).expect("undefined argument")
    }

    /// Completes parsing and validates arguments.
    ///
    /// **Note:** This function should be called before [`Arg::finish`] in
    /// [`Parser::finish`] to ensure all arguments are valid.
    pub fn finish(self) -> Result<()> {
        self.rt.borrow_mut().finish(&self)
    }
}

/// Builder for [`ParserContext`].
#[derive(Default)]
pub struct ParserContextBuilder {
    node: Option<Span>,
    args: Vec<&'static str>,
    formatter: Option<Box<dyn ErrorFormatter>>,
    rt: Rt,
}

impl ParserContextBuilder {
    /// Sets the node to which arguments belong.
    pub fn node(self, node: Span) -> Self {
        Self {
            node: Some(node),
            ..self
        }
    }

    /// Sets the error formatter. The default is [`DefaultFormatter`].
    pub fn formatter(self, formatter: impl 'static + ErrorFormatter) -> Self {
        Self {
            formatter: Some(Box::new(formatter)),
            ..self
        }
    }

    /// Registers an argument.
    pub fn arg<T>(&mut self, arg: &'static str) -> Arg<T> {
        self.args.push(arg);
        Arg::new(Id(self.args.len() - 1), self.rt.clone())
    }

    /// Consumes the builder and constructs [`ParserContext`].
    ///
    /// # Panics
    ///
    /// Panics if [`node`] is not supplied.
    ///
    /// [`node`]: Self::node
    pub fn build(self) -> ParserContext {
        let Self {
            node,
            args,
            formatter,
            rt,
        } = self;
        ParserContext {
            node: node.expect("`ParserContext::node` is required"),
            args,
            formatter: formatter.unwrap_or_else(|| Box::new(DefaultFormatter::default())),
            rt,
        }
    }
}
