use crate::{
    eval,
    gc::{
        GCString, GCVec, GarbageCollector,
        collection::{GCRoot, RootsRef},
    },
};

#[test]
fn string_list_test() {
    let mut gc = GarbageCollector::new();
    let root = GCRoot::new_empty();

    let expected = ["alpha", "beta", "c", "abcdefghijklmnopqrstuvwxyz"];
    let list = create_string_list(&expected, &mut gc, &root);

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

fn create_string_list<'r>(
    strings: &[&'static str],
    gc: &mut GarbageCollector,
    parent_root: RootsRef<'r>,
) -> eval::MValue {
    let mut root = parent_root.new();

    for s in strings {
        root.push(create_string_val(s, gc, &root));
    }

    create_string_val("fffffffffffffffffffffffffffffffffffffffff", gc, &root);
    // We don't use this value, so it should be freed next cycle

    let vec = unsafe { GCVec::new(gc, root.as_mut_mvalue_slice()) };

    let mut root = parent_root.new();
    root.push(vec);

    let old_ptr = vec.ptr();

    unsafe { gc.force_collect(&mut root) };

    let vec = root.get_value(0).unwrap();

    let eval::MValue::List(inner) = vec else {
        panic!()
    };

    // Test that the `GCVec` was moved. This indicates that the unused string was not copied during the GC cycle.
    assert_ne!(old_ptr, inner.ptr());

    vec
}

fn create_string_val<'r>(val: &str, gc: &mut GarbageCollector, root: RootsRef<'r>) -> eval::MValue {
    let string = GCString::new(gc, val);

    let mut root = root.new();
    root.push(string);

    unsafe { gc.force_collect(&mut root) };

    root.get_value(0).unwrap()
}
