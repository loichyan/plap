use crate::parser2::{AnyArg, Parser};

pub(crate) fn validate(parser: Parser) -> syn::Result<()> {
    let Parser {
        schema: _,
        values: _,
        errors,
    } = parser;

    errors.fail()
}

trait AnyArgExt: AnyArg {}

impl<T: ?Sized + AnyArg> AnyArgExt for T {}
