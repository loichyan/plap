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

#[macro_export]
macro_rules! define_args {
    ($(#[$attr:meta])*
    $vis:vis struct $name:ident {$(
        $(#[doc = $doc:literal])*
        $(#[plap($($plap:ident $(= $plap_val:expr)?),* $(,)?)])*
        $f_vis:vis $f_name:ident : $f_ty:ty,
    )*}) => {
        $(#[$attr])*
        $vis struct $name {$(
            $(#[doc = $doc])*
            $f_vis $f_name : $f_ty,
        )*}

        impl $crate::private::Args for $name {
            #[allow(unused_mut)]
            fn schema() -> $crate::private::Schema {
                let mut schema = $crate::private::Schema::default();
                $($crate::private::schema::register_to::<$f_ty>(
                    &mut schema,
                    $crate::private::Id::from(stringify!($f_name)),
                    {
                        let mut $f_name = $crate::private::schema::new::<$f_ty>();
                        $($f_name.help($doc);)*
                        $($($f_name.$plap($($plap_val)*);)*)*
                        $f_name
                    },
                );)*
                schema
            }

            fn init(schema: &$crate::private::Schema) -> Self {
                Self {
                    $($f_name: $crate::private::schema::init_from::<$f_ty>(
                        schema,
                        $crate::private::Id::from(stringify!($f_name)),
                    ),)*
                }
            }

            #[allow(unused_mut)]
            fn init_parser<'a>(
                schema: &'a $crate::private::Schema,
                args: &'a mut Self,
            ) -> $crate::private::Parser<'a> {
                let mut parser = $crate::private::Parser::new(schema);
                $($crate::private::schema::add_to_parser::<$f_ty>(
                    &mut parser,
                    &mut args.$f_name,
                );)*
                parser
            }
        }
    };
}
