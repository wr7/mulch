use std::mem;

use crate::{
    eval,
    gc::{
        GCString, GCVec,
        safety::{GC, GCCtx, GCRootGuard, let_gc_and_context, rebind, root},
    },
};

#[test]
fn string_list_test() {
    let_gc_and_context!(gc, ctx);

    let expected = ["alpha", "beta", "c", "abcdefghijklmnopqrstuvwxyz"];
    let list = create_string_list(&mut ctx, &expected);

    let eval::MValue::List(list) = list else {
        panic!()
    };

    let list = unsafe { list.as_slice(&gc) };

    assert_eq!(list.len(), expected.len());

    for (i, s) in list.iter().enumerate() {
        let eval::MValue::String(gcstr) = s else {
            panic!()
        };

        assert_eq!(unsafe { gcstr.get(&gc) }, expected[i]);
    }
}

fn create_string_list(ctx: &mut GCCtx, strings: &[&'static str]) -> eval::MValue {
    let string_roots: Vec<GCRootGuard<eval::MValue>> = strings
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

    eval::MValue::List(vec_root.get(ctx).raw())
}

fn create_string_val<'gc, 'c>(ctx: &'c mut GCCtx<'gc>, val: &str) -> GC<'c, eval::MValue> {
    let string = root!(ctx, GCString::new(ctx, val));

    ctx.force_collect();

    let val = eval::MValue::String(string.get(ctx).raw());

    unsafe { GC::new(ctx, val) }
}
