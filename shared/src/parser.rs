use crate::{runtime::Rt, Arg, DefaultFormatter, ErrorFormatter, Name};
use proc_macro2::{Span, TokenTree};
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
            let span = input.span();
            match self.parse_once(input) {
                Ok(true) => {}
                Ok(false) => {
                    let context = self.context();
                    let msg = if input.peek(Ident) {
                        let ident = input.parse::<Ident>()?;
                        context.format(&crate::Error::UnknownArg {
                            this: &ident.to_string(),
                        })
                    } else {
                        context.format(&crate::Error::InvalidInput)
                    };
                    context.rt.borrow_mut().add_error(span, msg);
                }
                Err(e) => {
                    self.context().rt.borrow_mut().add_syn_error(e);
                }
            }

            // Eat all tokens util a comma.
            loop {
                if input.is_empty() || input.parse::<Option<Token![,]>>()?.is_some() {
                    break;
                } else {
                    input.parse::<TokenTree>()?;
                }
            }
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
    formatter: Box<dyn ErrorFormatter>,
    rt: Rt,
}

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
    pub fn arg<T>(&mut self, name: Name) -> Arg<T> {
        let id = self.rt.borrow_mut().register(name);
        Arg::new(id, self.rt.clone())
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
            formatter,
            rt,
        } = self;
        ParserContext {
            node: node.expect("`ParserContext::node` is required"),
            formatter: formatter.unwrap_or_else(|| Box::new(DefaultFormatter::default())),
            rt,
        }
    }
}
