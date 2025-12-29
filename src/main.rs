use error::{SourceDB, dresult_unwrap};
use indoc::indoc;
use parser::binary::Op;

// TODO:
//  - create "GCDebug" trait for displaying garbage-collected values
//  - create `GCValue` for `GCRoot` type

pub mod error;
pub mod eval;
pub mod gc;
pub mod lexer;
pub mod parser;

mod util;

pub fn main() {
    let db = SourceDB::new();

    let source = indoc! {"string.push_str[\"howdy!\"]"};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let ast = dresult_unwrap(parser::parse_expression(&tokens, file_id), &db);

    println!("{:#?}", ast.unwrap());
}
