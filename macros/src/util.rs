macro_rules! syn_error {
    ($span:expr, $fmt:literal, $($args:tt)+) => {
        ::syn::Error::new($span, format!($fmt, $($args)*))
    };
    ($span:expr, $msg:expr $(,)?) => {
        ::syn::Error::new($span, $msg)
    };
}
