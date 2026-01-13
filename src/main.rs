use error::{SourceDB, dresult_unwrap};
use indoc::indoc;
use parser_old::binary::Op;

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

    let source = indoc! {"string.push_str[\"howdy!\"]"};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let ast = dresult_unwrap(parser_old::parse_expression(&tokens, file_id), &db);

    println!("{:#?}", ast.unwrap());
}
