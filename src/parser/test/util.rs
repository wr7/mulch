macro_rules! parse_test {
    ($test_name:ident, $src:expr, $($expected_ast:tt)+) => {
        #[test]
        fn $test_name() {
            let db = $crate::error::SourceDB::new();
            db.add(
                format!("{}.mulch", ::core::stringify!($test_name)).into(),
                $src.into(),
            );

            let gc = $crate::gc::GarbageCollector::new();
            let parser = $crate::parser::Parser::new_default(&gc);

            let tokens = $crate::dresult_unwrap($crate::lexer::Lexer::new($src, 0).lex(), &db);

            let expr  = $crate::pdresult_unwrap(
                <$crate::parser::ast::Expression as $crate::parser::Parse>::parse(&parser, &tokens),
                0,
                &db,
            ).unwrap();

            let ast = $crate::parser::test::util::ast!(&gc, $($expected_ast)+);

            let expr = unsafe { crate::gc::util::GCWrap::new(expr, &gc) };
            let ast = unsafe { crate::gc::util::GCWrap::new(ast, &gc) };

            if expr != ast {
                panic!(
                    "Parsed expression does not match expected expression. Got:\n{expr:#?}",
                );
            }
        }
    };
}

macro_rules! ast {
    {$gc:expr,
        $name:ident $args:tt
    } => {
        unsafe {if false {::core::hint::unreachable_unchecked()}; $crate::parser::test::util::_ast_impl!($gc, $name $args)}
    }
}

macro_rules! _ast_impl {
    {$gc:expr,
        PartialSpanned(
            $val_name:ident $val_args:tt,
            $span:expr $(,)?
        )
    } => {
        $crate::error::PartialSpanned(
            $crate::parser::test::util::_ast_impl!($gc, $val_name $val_args),
            ::copyspan::Span::from($span)
        )
    };

    {$gc:expr,
        Variable(
            $lit:literal $(,)?
        )
    } => {
        $crate::parser::ast::Expression::Variable(
            $crate::parser::Ident(
                $crate::gc::GCString::new($gc, $lit)
            )
        )
    };

    {$gc:expr,
        NumericLiteral(
            $num:literal / $den:literal $(,)?
        )
    } => {
        $crate::parser::ast::Expression::NumericLiteral(
            $crate::parser::ast::NumberLiteral(
                $crate::gc::GCNumber::parse_from_numerator_and_denominator_panicking($gc, ::core::stringify!($num), Some(::core::stringify!($den)))
            )
        )
    };

    {$gc:expr,
        NumericLiteral(
            $num:literal $(,)?
        )
    } => {
        $crate::parser::ast::Expression::NumericLiteral(
            $crate::parser::ast::NumberLiteral(
                $crate::gc::GCNumber::parse_from_numerator_and_denominator_panicking($gc, ::core::stringify!($num), None)
            )
        )
    };

    {$gc:expr,
        StringLiteral(
            $str:literal $(,)?
        )
    } => {
        $crate::parser::ast::Expression::StringLiteral(
            $crate::parser::ast::StringLiteral(
                $crate::gc::GCString::new($gc, $str)
            )
        )
    };

    {$gc:expr,
        Set [
            $(
                NamedValue $named_value_args:tt
            ),* $(,)?
        ]
    } => {
        $crate::parser::ast::Expression::Set(
            $crate::parser::ast::Set (
                $crate::parser::Bracketed($crate::parser::SeparatedList::from(
                    $crate::gc::GCVec::new($gc, &[
                        $(
                            $crate::parser::test::util::_ast_impl!($gc, NamedValue $named_value_args)
                        ),*
                    ])
                ))
            )
        )
    };

    {$gc:expr,
        List [
            $(
                $element_name:ident $element_args:tt
            ),* $(,)?
        ]
    } => {
        $crate::parser::ast::Expression::List(
            $crate::parser::ast::List (
                $crate::parser::Bracketed($crate::parser::SeparatedList::from(
                    $crate::gc::GCVec::new($gc, &[
                        $(
                            $crate::parser::test::util::_ast_impl!($gc, $element_name $element_args)
                        ),*
                    ])
                ))
            )
        )
    };

    {$gc:expr,
        LetIn {
            variables: [$(
                NamedValue $variable:tt
            ),*$(,)?],
            val: $val_name:ident $val_args:tt $(,)?
        }
    } => {
        $crate::parser::ast::Expression::LetIn(
            $crate::parser::ast::LetIn {
                let_: $crate::parser::keyword!("let")(),
                variables: $crate::gc::GCVec::<$crate::parser::ast::NamedValue>::new($gc, &[$(
                    $crate::parser::test::util::_ast_impl!($gc, NamedValue $variable)
                ),*]).into(),
                in_: $crate::parser::keyword!("in")(),
                val: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $val_name $val_args))
            }
        )
    };

    {$gc:expr,
        WithIn {
            variables: $variables:ident $variables_args:tt,
            val: $val_name:ident $val_args:tt $(,)?
        }
    } => {
        $crate::parser::ast::Expression::WithIn(
            $crate::parser::ast::WithIn {
                with_: $crate::parser::keyword!("with")(),
                variables: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $variables $variables_args)),
                in_: $crate::parser::keyword!("in")(),
                val: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $val_name $val_args))
            }
        )
    };

    {$gc:expr,
        Lambda {
            args: [
                $($arg_name:ident $arg_args:tt),* $(,)?
            ],
            expr: $expr_name:ident $expr_args:tt
            $(,)?
        }
    } => {
        $crate::parser::ast::Expression::Lambda(
            $crate::parser::ast::Lambda {
                args: $crate::parser::ast::lambda::Arguments(
                    $crate::parser::Bracketed(
                        $crate::parser::SeparatedList::from($crate::gc::GCVec::new($gc, &[
                            $(
                                $crate::parser::test::util::_ast_impl!($gc, $arg_name $arg_args)
                            ),*
                        ]))
                    )
                ),
                arrow: $crate::parser::punct!("->")(),
                expr: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $expr_name $expr_args))
            }
        )
    };

    {$gc:expr,
        BinaryOperation {
            lhs: $lhs_name:ident $lhs_args:tt,
            operator: $operator:ident,
            rhs: $rhs_name:ident $rhs_args:tt
            $(,)?
        }
    } => {
        $crate::parser::ast::Expression::BinaryOperation(
            $crate::parser::ast::BinaryOperation {
                lhs: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $lhs_name $lhs_args)),
                operator: $crate::parser::ast::operation::BinaryOperator::$operator,
                rhs: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $rhs_name $rhs_args))
            }
        )
    };

    {$gc:expr,
        UnaryOperation {
            operator: $operator:ident,
            arg: $arg_name:ident $arg_args:tt
            $(,)?
        }
    } => {
        $crate::parser::ast::Expression::UnaryOperation(
            $crate::parser::ast::UnaryOperation {
                operator: $crate::parser::ast::operation::UnaryOperator::$operator,
                arg: $crate::gc::GCBox::new($gc, $crate::parser::test::util::_ast_impl!($gc, $arg_name $arg_args))
            }
        )
    };

    {$gc:expr,
        FunctionCall {
            function: $function:ident $function_args:tt,
            args: FunctionCallArgs [$(
                $arg:ident $arg_args:tt
            ),* $(,)?]
            $(,)?
        }
    } => {
        $crate::parser::ast::Expression::FunctionCall(
            $crate::parser::ast::FunctionCall {
                function: $crate::gc::GCBox::new($gc,
                    $crate::parser::test::util::_ast_impl!($gc, $function $function_args)
                ),
                args: $crate::parser::ast::FunctionCallArgs($crate::parser::Bracketed(
                    $crate::parser::SeparatedList::from($crate::gc::GCVec::new($gc, &[
                        $(
                            $crate::parser::test::util::_ast_impl!($gc, $arg $arg_args)
                        ),*
                    ]))
                ))
            }
        )
    };

    {$gc:expr,
        SingleArgument {
            name: $name:literal,
            default_value: None $(,)?
        }
    } => {
        $crate::parser::ast::lambda::Argument::Single(
            $crate::parser::ast::lambda::SingleArgument {
                name: $crate::parser::IdentOrString($crate::gc::GCString::new($gc, $name)),
                default_value: None
            }
        )
    };

    {$gc:expr,
        SingleArgument {
            name: $name:literal,
            default_value: Some(ArgDefaultValue $default_args:tt) $(,)?
        }
    } => {
        $crate::parser::ast::lambda::Argument::Single(
            $crate::parser::ast::lambda::SingleArgument {
                name: $crate::parser::IdentOrString($crate::gc::GCString::new($gc, $name)),
                default_value: Some($crate::parser::test::util::_ast_impl!($gc, ArgDefaultValue $default_args))
            }
        )
    };

    {$gc:expr,
        NamedValue {
            name: PartialSpanned(
                $name:literal,
                $name_span:expr $(,)?
            ),
            value: $val_name:ident $val_args:tt $(,)?
        }
    } => {
        $crate::parser::ast::NamedValue {
            name: $crate::error::PartialSpanned(
                $crate::parser::IdentOrString($crate::gc::GCString::new($gc, $name)),
                ::copyspan::Span::from($name_span)
            ),
            eq_: $crate::parser::punct!("=")(),
            value: $crate::parser::test::util::_ast_impl!($gc, $val_name $val_args)
        }
    };
}

#[allow(unused)]
pub(crate) use ast;

pub(crate) use parse_test;

#[doc(hidden)]
#[allow(unused)]
pub(crate) use _ast_impl;
