#![allow(clippy::enum_clike_unportable_variant)]
#![allow(clippy::type_complexity)]

use error::{SourceDB, dresult_unwrap};
use indoc::indoc;
use parser_old::binary::Op;

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
pub mod parser_old;

mod util;

pub fn main() {
    let db = SourceDB::new();

    let source = indoc! {"[a, b, {c = f; \"hello\" = b;}, {}, []]"};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let gc = GarbageCollector::new();
    let parser = Parser::new_default(&gc);

    let ast = pdresult_unwrap(parser::ast::Expression::parse(&parser, &tokens), 0, &db);
    let ast = unsafe { ast.unwrap().wrap(&gc) };

    dbg!(ast);
}
