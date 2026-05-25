use std::mem;

use crate::{
    eval,
    gc::{GCString, GCVec, GarbageCollector, roots::GCRootGuard},
};

#[test]
fn string_list_test() {
    let gc = GarbageCollector::new();

    let expected = ["alpha", "beta", "c", "abcdefghijklmnopqrstuvwxyz"];
    let list = create_string_list(&expected, &gc);

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

fn create_string_list<'r>(strings: &[&'static str], gc: &GarbageCollector) -> eval::MValue {
    let string_roots: Vec<GCRootGuard<eval::MValue>> = strings
        .iter()
        .map(|s| unsafe {
            let string = create_string_val(s, gc);
            gc.push_root(string)
        })
        .collect();

    create_string_val("fffffffffffffffffffffffffffffffffffffffff", gc);
    // We don't use this value, so it should be freed next cycle

    let vec_root = {
        let vec = unsafe {
            GCVec::from_iter_and_len(gc, string_roots.iter().map(|r| r.get()), strings.len())
        };

        for r in string_roots.into_iter().rev() {
            mem::drop(r);
        }

        unsafe { gc.push_root(vec) }
    };

    let old_ptr = unsafe { vec_root.get().ptr() };

    unsafe { gc.force_collect() };

    // Test that the `GCVec` was moved. This indicates that the unused string was not copied during the GC cycle.
    assert_ne!(old_ptr, unsafe { vec_root.get().ptr() });

    eval::MValue::List(unsafe { vec_root.get() })
}

fn create_string_val<'r>(val: &str, gc: &GarbageCollector) -> eval::MValue {
    let string = GCString::new(gc, val);

    let string_root = unsafe { gc.push_root(string) };

    unsafe { gc.force_collect() };

    eval::MValue::String(unsafe { string_root.get() })
}
