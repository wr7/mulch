use mulch_macros::{GCDebug, GCProject, GCPtr, gc_fn};

use crate::{
    error::{DResult, FullSpan, Spanned},
    eval::{self, MValue, Scope, evaluate},
    gc::{
        GCBox, GCRootRef,
        safety::{GC, GCCtx, Projected, gc_args, rebind, root},
    },
    parser::ast,
};

#[derive(Clone, Copy, GCPtr, GCDebug, GCProject)]
struct UnevaluatedLazyValue {
    ast: Spanned<ast::Expression>,
    scope: GCBox<Scope>,
}

#[derive(Clone, Copy, GCPtr, GCDebug, GCProject)]
#[repr(usize)]
#[msb_reserved]
enum LazyValueData {
    Unevaluated(UnevaluatedLazyValue),
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
    pub fn from_ast<'c>(
        ctx: &'c GCCtx,
        ast: GC<'c, Spanned<ast::Expression>>,
        scope: GC<'c, GCBox<Scope>>,
    ) -> GC<'c, Self> {
        let unevaluated = Projected::<UnevaluatedLazyValue> { ast, scope };

        let data: GC<LazyValueData> =
            Projected::<LazyValueData>::Unevaluated(unevaluated.into()).into();

        let inner = GCBox::new(data);

        // SAFETY: we know that `inner` is currently valid because it's wrapped in `GC`
        unsafe { GC::new(ctx, Self { inner: inner.raw() }) }
    }

    #[gc_fn]
    pub fn get_or_evaluate<'gc, 'c>(
        ctx: &'c mut gc!('gc, value: Self),
        usage_span: FullSpan,
    ) -> DResult<GC<'c, MValue>> {
        let inner = unsafe { GC::new(ctx, value.raw().inner) };

        match inner.get().project() {
            Projected::<LazyValueData>::Evaluated(mvalue) => return Ok(rebind!(ctx, mvalue)),
            Projected::<LazyValueData>::CurrentlyBeingEvaluated(definition_span) => Err(
                eval::error::illegal_recursively_defined_value(definition_span.raw(), usage_span),
            ),
            Projected::<LazyValueData>::Unevaluated(unevaluated_data) => {
                let ast = unevaluated_data.project().ast;
                let scope = unevaluated_data.project().scope.get();

                unsafe {
                    inner
                        .raw()
                        .as_mut(ctx)
                        .write(LazyValueData::CurrentlyBeingEvaluated(ast.project().1))
                };

                let inner_root = root!(ctx, inner);

                let value: GC<'c, MValue> = rebind!(ctx, evaluate(gc_args!(ctx, ast, scope))?);

                let inner = inner_root.get(ctx);

                unsafe {
                    inner
                        .raw()
                        .as_mut(ctx)
                        .write(LazyValueData::Evaluated(value.raw()));
                }

                Ok(value)
            }
        }
    }
}

impl GCRootRef<LazyValue> {}
