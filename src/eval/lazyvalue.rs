use mulch_macros::{GCDebug, GCProject, GCPtr, gc_fn};

use crate::{
    error::{DResult, FullSpan, Spanned},
    eval::{self, MValue, evaluate},
    gc::{
        GCBox, GCRootRef,
        safety::{GC, GCCtx, Projected, gc_args, rebind, root},
    },
    parser::ast,
};

#[derive(Clone, Copy, GCPtr, GCDebug, GCProject)]
#[repr(usize)]
#[msb_reserved]
enum LazyValueData {
    Unevaluated(Spanned<ast::Expression>),
    Evaluated(MValue),
    CurrentlyBeingEvaluated(FullSpan),
}

/// A lazily-evaluated value. This is used for things like scopes or attribute sets.
#[derive(Clone, Copy, GCPtr, GCDebug)]
#[debug_direct]
pub struct LazyValue {
    inner: GCBox<LazyValueData>,
}

impl LazyValue {
    pub fn from_ast<'c>(ctx: &'c GCCtx, ast: GC<'c, Spanned<ast::Expression>>) -> GC<'c, Self> {
        let data: GC<LazyValueData> = Projected::<LazyValueData>::Unevaluated(ast).into();

        let inner = GCBox::new(ctx, data);

        // SAFETY: we know that `inner` is currently valid because it's wrapped in `GC`
        unsafe { GC::new(ctx, Self { inner: inner.raw() }) }
    }

    #[gc_fn]
    pub fn get_or_evaluate<'gc, 'c>(
        ctx: &'c mut gc!(
                'gc,
                value: Self,
                usage_span: FullSpan
                ),
    ) -> DResult<GC<'c, MValue>> {
        let inner = unsafe { GC::new(ctx, value.raw().inner) };

        match inner.get().project() {
            Projected::<LazyValueData>::Evaluated(mvalue) => return Ok(rebind!(ctx, mvalue)),
            Projected::<LazyValueData>::CurrentlyBeingEvaluated(definition_span) => {
                Err(eval::error::illegal_recursively_defined_value(
                    definition_span.raw(),
                    usage_span.raw(),
                ))
            }
            Projected::<LazyValueData>::Unevaluated(ast) => {
                unsafe {
                    inner
                        .raw()
                        .as_mut(ctx)
                        .write(LazyValueData::CurrentlyBeingEvaluated(
                            ast.clone().project().1,
                        ))
                };

                let inner_root = root!(ctx, inner);

                let value: GC<'c, MValue> = rebind!(ctx, evaluate(gc_args!(ctx, ast))?);

                let inner = inner_root.get(ctx);

                unsafe {
                    inner
                        .raw()
                        .as_mut(ctx)
                        .write(LazyValueData::Evaluated(value.clone().raw()));
                }

                Ok(value)
            }
        }
    }
}

impl GCRootRef<LazyValue> {}
