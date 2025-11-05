use error::{SourceDB, dresult_unwrap};
use indoc::indoc;
use parser::binary::Op;

pub mod error;
pub mod lexer;
pub mod parser;

mod util;

pub fn main() {
    let db = SourceDB::new();

    let source = indoc! {"{foo, bar} -> biz"};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    let ast = dresult_unwrap(parser::parse_expression(&tokens, file_id), &db);

    println!("{:#?}", ast.unwrap());
}
