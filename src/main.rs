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
        let
            a = "0";
            b = "1";
        in
        [a, b]
    "#};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap!(lexer::Lexer::new(source, file_id).lex(), &db);

    let ast = dresult_unwrap!(parser::parse_expression(&tokens, file_id), &db);

    dbg!(ast);
}
