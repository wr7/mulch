#![allow(clippy::enum_clike_unportable_variant)]
#![allow(clippy::type_complexity)]

use error::{SourceDB, dresult_unwrap};

use crate::{
    error::{PartialSpanned, pdresult_unwrap},
    eval::evaluate,
    gc::{
        GCPtr,
        safety::{GC, gc_args, let_gc_and_context},
    },
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
// - Remove UnsafeRootGuard
// - Add `phantom` annotations
// - Use `zst` annotations for `GCPtr` optimizations
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
// - Add `expected [TypeA]; got [TypeBA]` error messages.
// - Add logic for printing recursively-defined values.

pub fn main() {
    let db = SourceDB::new();

    let source = "{x = \"my_x_value\"; sub_set = {val_a = \"my_a_value\";}}.sub_set.val_a";

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let_gc_and_context!(gc, ctx);

    let parser = Parser::new_default(&gc);

    let ast = pdresult_unwrap(
        PartialSpanned::<parser::ast::Expression>::parse(&parser, &tokens),
        0,
        &db,
    )
    .unwrap();

    unsafe { dbg!(ast.wrap(&gc)) };

    let ast = unsafe { GC::new(ctx, ast) };

    let value = dresult_unwrap(evaluate(gc_args!(ctx, ast.with_file_id(0))), &db);

    dbg!(value);
}
