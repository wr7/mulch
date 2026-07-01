use mulch_macros::gc_fn;

use crate::{
    error::{DResult, PartialSpanned, Spanned},
    eval::{self, MValue},
    gc::{
        GCVec,
        safety::{GC, GCRootGuard, Projected, gc_args, rebind},
    },
    parser::ast,
};

#[gc_fn]
pub(super) fn evaluate_list<'c>(
    ctx: &'c mut gc!(ast: Spanned<ast::List>),
) -> DResult<GC<'c, MValue>> {
    let ast_span = ast.project().1;
    let ast = ast.project().0;

    let elem_asts: GC<GCVec<PartialSpanned<ast::Expression>>> =
        ast.project().0.project().0.project().values;

    // Create roots for the element ASTs //
    //
    // We create individual roots so that we can discard the roots of elements that we've already evaluated.
    let mut elem_ast_roots: Vec<GCRootGuard<PartialSpanned<ast::Expression>>> =
        Vec::with_capacity(elem_asts.len());

    for elem_ast in elem_asts.iter() {
        // PANIC NOTE: We free these roots in reverse order later in this function.
        //
        // All roots created after this are freed first
        elem_ast_roots.push(GCRootGuard::new(ctx, elem_ast));
    }

    // Evaluate the element ASTs //
    let mut elem_value_roots: Vec<GCRootGuard<MValue>> = Vec::with_capacity(elem_asts.len());

    for elem_ast_root in elem_ast_roots.iter() {
        let elem_ast = elem_ast_root.get(ctx).with_file_id(ast_span.file_id);

        let elem_value = rebind!(ctx, eval::evaluate(gc_args!(ctx, elem_ast))?);

        elem_ast_root.remove();

        // PANIC NOTE: We free these roots in reverse order later in this function.
        //
        // There are no other roots created before this is freed.
        elem_value_roots.push(GCRootGuard::new(ctx, elem_value));
    }

    // Create the actual value //
    let output_val: GC<GCVec<MValue>> = GCVec::from_iter_and_len(
        ctx,
        elem_value_roots.iter().map(|val_root| val_root.get(ctx)),
        elem_value_roots.len(),
    );

    // Drop the value roots in reverse order //
    for elem_value_root in elem_value_roots.into_iter().rev() {
        std::mem::drop(elem_value_root);
    }

    // Drop the AST roots in reverse order //
    for elem_ast_root in elem_ast_roots.into_iter().rev() {
        std::mem::drop(elem_ast_root);
    }

    Ok(Projected::<MValue>::List(output_val).into())
}
