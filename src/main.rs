use error::{SourceDB, dresult_unwrap};
use indoc::indoc;

pub mod error;
pub mod lexer;
pub mod parser;

mod util;

pub fn main() {
    let db = SourceDB::new();

    let source = indoc! {r#"
        add(a, 1)
    "#};

    let file_id = db.add("main.mulch".into(), source.to_owned());

    let tokens = dresult_unwrap(lexer::Lexer::new(source, file_id).lex(), &db);

    dbg!(&tokens);

    let ast = dresult_unwrap(parser::parse_expression(&tokens, file_id), &db);

    dbg!(ast);
}
