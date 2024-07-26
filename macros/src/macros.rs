macro_rules! syn_error {
    ($span:expr, $fmt:literal, $($args:tt)+) => {
        ::syn::Error::new($span, format!($fmt, $($args)*))
    };
    ($span:expr, $msg:expr $(,)?) => {
        ::syn::Error::new($span, $msg)
    };
}

macro_rules! define_value_enum {
    ($(#[$attr:meta])*
    $vis:vis enum $name:ident {
        $($variant:ident,)*
    }) => {
        $(#[$attr])*
        $vis enum $name {
            $($variant(::proc_macro2::Span),)*
        }

        impl $name {
            pub fn span(&self) -> ::proc_macro2::Span {
                match self {
                    $($name::$variant(v) => *v,)*
                }
            }
        }

        impl ::syn::parse::Parse for $name {
            fn parse(input: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
                let lookahead = input.lookahead1();
                $(::syn::custom_keyword!($variant);
                if lookahead.peek($variant) {
                    return Ok(Self::$variant(input.parse::<$variant>()?.span));
                })*
                Err(lookahead.error())
            }
        }
    };
}
