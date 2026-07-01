use itertools::Itertools;
use mulch_macros::{GCDebug, GCProject, GCPtr, gc_fn};

use crate::{
    error::{DResult, Spanned},
    eval::{self, MValue, lazyvalue::LazyValue},
    gc::{
        GCString, GCVec,
        safety::{GC, Projected, gc_args, rebind, root},
    },
    parser::ast::{self, MemberAccess, NamedValue},
};

#[derive(Clone, Copy, GCDebug, GCPtr)]
pub struct Set {
    values: GCVec<NamedMValue>,
}

impl<'c> GC<'c, Set> {
    pub fn get_attr(self, attr_name: &str) -> Option<GC<'c, LazyValue>> {
        let gc = self.gc();

        let result_idx = unsafe {
            self.raw()
                .values
                .as_slice(gc)
                .binary_search_by_key(&attr_name, |attr| attr.name.0.get(gc))
        };

        let values = unsafe { GC::from_raw_parts(gc, self.raw().values) };

        result_idx
            .ok()
            .and_then(|idx| values.get(idx))
            .map(|attr| attr.project().value)
    }
}

#[derive(Clone, Copy, GCDebug, GCPtr, GCProject)]
struct NamedMValue {
    name: Spanned<GCString>,
    value: LazyValue,
}

#[gc_fn]
pub(super) fn evaluate_set<'c>(
    ctx: &'c mut gc!(ast: Spanned<ast::Set>),
) -> DResult<GC<'c, MValue>> {
    let ast = ast.project();

    let ast_attributes: GC<GCVec<NamedValue>> = ast.0.project().0.project().0.project().values;

    let file_id = ast.1.file_id;

    let named_values = ast_attributes.iter().map(|attr| {
        Projected::<NamedMValue> {
            name: attr
                .project()
                .name
                .map(|id| id.project().0)
                .with_file_id(file_id),
            value: LazyValue::from_ast(ctx, attr.project().value.with_file_id(file_id)),
        }
        .into()
    });

    let out_attrs =
        GCVec::<NamedMValue>::from_iter_and_len(ctx, named_values, ast_attributes.len());

    // Sort attributes //

    // SAFETY: nothing else is borrowing from the memory that `out_attrs` is pointing to
    unsafe {
        let mut_slice =
            std::slice::from_raw_parts_mut(out_attrs.raw().as_mut_ptr(ctx), out_attrs.len());

        mut_slice.sort_by(|a, b| a.name.0.get(ctx).cmp(b.name.0.get(ctx)));
    }

    // Ensure that there are no duplicate attributes //
    for (prev, cur) in out_attrs.iter().tuple_windows() {
        let prev_name: GC<GCString> = prev.project().name.project().0;
        let cur_name: GC<GCString> = cur.project().name.project().0;

        if prev_name.read() == cur_name.read() {
            return Err(eval::error::attribute_defined_multiple_times(
                prev.project().name.project().1,
                cur.project().name.project().1,
            ));
        }
    }

    // SAFETY: we know that `out_attrs` is valid because it's wrapped in `GC`
    let set = unsafe {
        GC::new(
            ctx,
            Set {
                values: out_attrs.raw(),
            },
        )
    };

    Ok(Projected::<MValue>::Set(set).into())
}

#[gc_fn]
pub(super) fn evaluate_member_access<'c>(
    ctx: &'c mut gc!(ast: Spanned<MemberAccess>),
) -> DResult<GC<'c, MValue>> {
    let lazy_value;
    let ast_span;

    {
        let ast = ast.project();
        ast_span = ast.1;

        let ast = ast.0;

        let rhs: GC<GCString> = ast.project().rhs.project().0;
        let rhs = root!(ctx, rhs);

        let lhs = ast.project().lhs.get().with_file_id(ast_span.file_id);

        let lhs = rebind!(ctx, eval::evaluate(gc_args!(ctx, lhs))?);

        let Projected::<MValue>::Set(lhs) = lhs.project() else {
            return Err(eval::error::member_access_on_non_set(ast_span));
        };

        let rhs = rhs.get(ctx);

        lazy_value = match lhs.get_attr(rhs.read()) {
            Some(lazy_value) => lazy_value,
            None => return Err(eval::error::no_attribute_with_name(ast_span, rhs.read())),
        }
    };

    LazyValue::get_or_evaluate(gc_args!(ctx, lazy_value), ast_span)
}
