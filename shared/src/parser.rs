use crate::{arg::ArgGroup, runtime::Rt, Arg, DefaultFormatter, ErrorFormatter, Name};
use proc_macro2::{Span, TokenTree};
use syn::{parse::ParseStream, Ident, Result, Token};

/// Parse input stream into user-defined container.
pub trait Parser: Sized {
    type Output;

    /// Constructs a parser from the pre-configured context.
    fn from_context(context: ParserContext) -> Self;

    /// Returns the context of current parser.
    fn context(&self) -> &ParserContext;

    /// Returns the mutable context of current parser.
    fn context_mut(&mut self) -> &mut ParserContext;

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
                Ok(false) => {
                    let context = self.context_mut();
                    let msg = if input.peek(Ident) {
                        // Attempt to parse as an unknown argument
                        let name = input.parse::<Ident>()?;
                        context.format(&crate::Error::UnknownArg {
                            this: &name.to_string(),
                        })
                    } else {
                        // Invalid input
                        context.format(&crate::Error::InvalidInput)
                    };
                    context.error(syn::Error::new(span, msg));
                }
                // Report the error and eat all rest tokens
                Err(e) => self.context_mut().error(e),
                Ok(true) if input.is_empty() => break,
                // No errors,
                Ok(true) => match input.parse::<Token![,]>() {
                    Ok(_) => continue,
                    // expect a comma
                    Err(e) => self.context_mut().error(e),
                },
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

    /// Completes parsing, validates results and returns errors that occurred
    /// during parsing/validating.
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

    /// Registers an argument.
    pub fn arg<T>(&mut self, name: Name) -> Arg<T> {
        let id = self.rt.borrow_mut().register(name);
        Arg::new(id, self.rt.clone())
    }

    /// Registers a group.
    pub fn group(&mut self, name: Name) -> ArgGroup {
        let id = self.rt.borrow_mut().register(name);
        ArgGroup::new(id, self.rt.clone())
    }

    /// Saves an error which will be reported in [`finish`].
    ///
    /// [`finish`]: Self::finish
    pub fn error(&mut self, e: syn::Error) {
        self.rt.borrow_mut().add_error(e);
    }

    /// Completes parsing and validates arguments.
    ///
    /// **Note:** This function should be called before accessing the value(s)
    /// of an [`Arg`] in [`Parser::finish`] to ensure all arguments are valid.
    pub fn finish(self) -> Result<()> {
        self.rt.take().finish(self)
    }
}

/// Builder for [`ParserContext`].
#[derive(Default)]
pub struct ParserContextBuilder {
    node: Option<Span>,
    namespace: Option<Name>,
    formatter: Option<Box<dyn ErrorFormatter>>,
}

impl ParserContextBuilder {
    /// Sets the node to which arguments belong.
    pub fn node(self, node: Span) -> Self {
        Self {
            node: Some(node),
            ..self
        }
    }

    /// Defines the namespace of [`DefaultFormatter`] for arguments and formats
    /// each argument as `namespace.argument`.
    pub fn namespace(self, namespace: Name) -> Self {
        Self {
            namespace: Some(namespace),
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
            namespace,
            formatter,
        } = self;
        ParserContext {
            node: node.expect("`ParserContext::node` is required"),
            formatter: formatter.unwrap_or_else(|| Box::new(DefaultFormatter { namespace })),
            rt: <_>::default(),
        }
    }
}
