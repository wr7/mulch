#![allow(clippy::enum_clike_unportable_variant)]
#![allow(clippy::type_complexity)]

use error::{SourceDB, dresult_unwrap};

use crate::{
    error::pdresult_unwrap,
    gc::{GCPtr, GarbageCollector},
    parser::{Parse, Parser},
};

extern crate self as mulch;

pub mod error;
pub mod eval;
pub mod gc;
pub mod lexer;
pub mod parser;

mod util;

// TODO:
// - Add more parser tests for:
//     - Set and list lambda arguments
//     - Default lambda arguments and argument bindings
//     - Method calls
//     - Member access
// - Remove dependence on GMP. This will allow MIRI to run and will more easily allow the
//   implementation of certain algorithms not in `mpn`.
//     - Div_exact (for `reduce` function)
//     - `div_by_constant` for radix conversion
//     - This will allow use to remove `#[cfg(any(not(miri), rust_analyzer))]` from several tests
// - Use high multiplication for computing required number of digits/limbs

pub fn main() {
    let db = SourceDB::new();

    let source = "let pi = x in e^(i * pi)";

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let gc = GarbageCollector::new();
    let parser = Parser::new_default(&gc);

    let ast = pdresult_unwrap(parser::ast::Expression::parse(&parser, &tokens), 0, &db);
    let ast = unsafe { ast.unwrap().wrap(&gc) };

    dbg!(ast);
}
