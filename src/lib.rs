#[macro_use]
mod macros;
mod id;
mod parser;
mod schema;
mod util;
mod validate;

#[doc(inline)]
pub use {
    id::Id,
    parser::{Arg, Group, Parser},
    schema::{ArgSchema, GroupSchema, Schema, SchemaFieldType},
};

pub trait Args: Sized {
    fn schema() -> Schema;

    fn init(schema: &Schema) -> Self;

    fn init_parser<'a>(&'a mut self, schema: &'a Schema) -> Parser<'a>;

    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let schema = Self::schema();
        let mut args = Self::init(&schema);
        let mut parser = args.init_parser(&schema);
        parser.parse(input)?;
        parser.finish()?;
        Ok(args)
    }
}

/// NON PUBLIC APIS.
#[doc(hidden)]
pub mod private {
    pub use crate::*;

    /// Helper functions to work with [`Schema`].
    pub mod schema {
        use super::*;

        pub fn new<T: SchemaFieldType>() -> T::Schema {
            T::Schema::default()
        }

        pub fn register_to<T: SchemaFieldType>(target: &mut Schema, name: Id, schema: T::Schema) {
            T::register_to(target, name, schema)
        }

        pub fn init_from<T: SchemaFieldType>(schema: &Schema, name: Id) -> T {
            T::init_from(schema, name)
        }

        pub fn add_to_parser<'a, T: SchemaFieldType>(parser: &mut Parser<'a>, slf: &'a mut T) {
            T::add_to_parser(parser, slf)
        }
    }
}
