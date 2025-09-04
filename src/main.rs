use error::SourceDB;
use indoc::indoc;

pub mod error;
pub mod lexer;
pub mod parser;

mod util;

// TODO: rewrite dresult_unwrap as a function

pub fn main() {
    let db = SourceDB::new();

    let source = indoc! {r#"
        313.2
    "#};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap!(lexer::Lexer::new(source, file_id).lex(), &db);

    dbg!(&tokens);

    let ast = dresult_unwrap!(parser::parse_expression(&tokens, file_id), &db);

    dbg!(ast);
}
