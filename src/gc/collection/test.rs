use std::mem;

use crate::{
    eval::MValue,
    gc::{
        GCString, GCVec,
        safety::{GC, GCCtx, GCRootGuard, Projected, let_gc_and_context, rebind, root},
    },
};

#[test]
fn string_list_test() {
    let_gc_and_context!(gc, ctx);

    let expected = ["alpha", "beta", "c", "abcdefghijklmnopqrstuvwxyz"];
    let list = rebind!(ctx, create_string_list(ctx, &expected));

    let Projected::<MValue>::List(list) = list.project() else {
        panic!()
    };

    let list: GC<GCVec<MValue>> = list;

    assert_eq!(list.len(), expected.len());

    for (i, s) in list.iter().enumerate() {
        let Projected::<MValue>::String(gcstr) = s.project() else {
            panic!()
        };

        assert_eq!(gcstr.read(), expected[i]);
    }
}

fn create_string_list<'c>(ctx: &'c mut GCCtx, strings: &[&'static str]) -> GC<'c, MValue> {
    let string_roots: Vec<GCRootGuard<MValue>> = strings
        .iter()
        .map(|s| {
            let string = rebind!(ctx, create_string_val(ctx, s));

            // SAFETY: we drop these guards in reverse order later in the function.
            // Notably, we do not create any more roots before then
            unsafe { GCRootGuard::new(ctx, string) }
        })
        .collect();

    create_string_val(ctx, "fffffffffffffffffffffffffffffffffffffffff");
    // We don't use this value, so it should be freed next cycle

    let vec = GCVec::from_iter_and_len(ctx, string_roots.iter().map(|r| r.get(ctx)), strings.len());

    for r in string_roots.into_iter().rev() {
        // Free the roots in order to uphold the safety guarentee above
        mem::drop(r);
    }

    let vec_root = root!(ctx, vec);

    let old_ptr = vec_root.get(ctx).raw().ptr();

    ctx.force_collect();

    // Test that the `GCVec` was moved. This indicates that the unused string was not copied during the GC cycle.
    assert_ne!(old_ptr, vec_root.get(ctx).raw().ptr());

    Projected::<MValue>::List(vec_root.get(ctx)).into()
}

fn create_string_val<'gc, 'c>(ctx: &'c mut GCCtx<'gc>, val: &str) -> GC<'c, MValue> {
    let string = root!(ctx, GCString::new(ctx, val));

    ctx.force_collect();

    Projected::<MValue>::String(string.get(ctx)).into()
}
