use crate::gc::{GCString, GarbageCollector};

#[test]
fn gcspace_string_test() {
    let strings = ["foo bar", "foo bar biz bang bazinga!", "foo bar biz bang"];
    let mut gc = GarbageCollector::new();

    let string0 = GCString::new(&mut gc, strings[0]);
    let string1 = GCString::new(&mut gc, strings[1]);
    let string2 = GCString::new(&mut gc, strings[2]);

    assert_eq!(string0.get_inline(), Some(strings[0]));
    assert_eq!(string1.get_inline(), None);
    assert_eq!(string2.get_inline(), None);

    assert_eq!(unsafe { string0.get(&gc) }, strings[0]);
    assert_eq!(unsafe { string1.get(&gc) }, strings[1]);
    assert_eq!(unsafe { string2.get(&gc) }, strings[2]);
}
