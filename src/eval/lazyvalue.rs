use mulch_macros::{GCDebug, GCPtr};

use crate::{
    error::{DResult, FullSpan, Spanned},
    eval::{self, Evaluator, MValue},
    gc::{GCBox, GCRootRef, GarbageCollector},
    parser::ast,
};

#[derive(Clone, GCPtr, GCDebug)]
#[repr(usize)]
#[msb_reserved]
enum LazyValueData {
    Unevaluated(Spanned<ast::Expression>),
    Evaluated(MValue),
    CurrentlyBeingEvaluated(FullSpan),
}

/// A lazily-evaluated value. This is used for things like scopes or attribute sets.
#[derive(Clone, GCPtr, GCDebug)]
#[debug_direct]
pub struct LazyValue {
    inner: GCBox<LazyValueData>,
}

impl LazyValue {
    pub unsafe fn from_ast(gc: &GarbageCollector, ast: Spanned<ast::Expression>) -> Self {
        let data = LazyValueData::Unevaluated(ast);

        Self {
            inner: unsafe { GCBox::new(gc, data) },
        }
    }

    pub unsafe fn to_ast(&self, gc: &GarbageCollector) -> Option<Spanned<ast::Expression>> {
        match unsafe { self.inner.get(gc) } {
            LazyValueData::Unevaluated(ast) => Some(ast),
            _ => None,
        }
    }

    pub unsafe fn get_or_evaluate(
        self,
        evaluator: &Evaluator,
        usage_span: FullSpan,
    ) -> DResult<MValue> {
        let data = unsafe { self.inner.get(evaluator.gc) };

        match data {
            LazyValueData::Evaluated(mvalue) => Ok(mvalue),
            LazyValueData::CurrentlyBeingEvaluated(definition_span) => Err(
                eval::error::illegal_recursively_defined_value(definition_span, usage_span),
            ),
            LazyValueData::Unevaluated(ast) => {
                unsafe {
                    self.inner
                        .as_mut(evaluator.gc)
                        .write(LazyValueData::CurrentlyBeingEvaluated(ast.1))
                };

                let inner_root = unsafe { evaluator.gc.push_root(self.inner) };

                let value = evaluator.evaluate(ast)?;

                unsafe {
                    inner_root
                        .get()
                        .as_mut(evaluator.gc)
                        .write(LazyValueData::Evaluated(value.clone()));
                }

                Ok(value)
            }
        }
    }
}

impl GCRootRef<LazyValue> {}
