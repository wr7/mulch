#![allow(clippy::enum_clike_unportable_variant)]
#![allow(clippy::type_complexity)]

use error::{SourceDB, dresult_unwrap};

use crate::{
    error::{PartialSpanned, pdresult_unwrap},
    eval::Evaluator,
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
// - Add logic to check for opportunities for use-after-free in debug mode:
//   - Add a "generation index" to GC primitives
//   - Add a "generation counter" to the garbage collector that increments each time a garbage-collection cycle can be performed.
//   - This process would be extremely slow, so we need to figure out a way to make this more performant.
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
// - Add more optimized `gc_root_entry` methods to `#[derive(GCPtr)]` for single-field structs.
// - Add proper documentation for the derive macros.
// - Add logic for printing recursively-defined values.

pub fn main() {
    let db = SourceDB::new();

    let source = "{x = \"my_x_value\"; sub_set = {val_a = \"a\";}}";

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let gc = GarbageCollector::new();
    let parser = Parser::new_default(&gc);

    let ast = pdresult_unwrap(
        PartialSpanned::<parser::ast::Expression>::parse(&parser, &tokens),
        0,
        &db,
    )
    .unwrap();

    let evaluator = Evaluator::new(&gc);
    let value = dresult_unwrap(evaluator.evaluate(ast.with_file_id(0)), &db);

    unsafe { dbg!(value.wrap(&gc)) };
}
