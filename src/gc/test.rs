use crate::gc::{GCString, safety::let_gc_and_context};

#[test]
fn gcspace_string_test() {
    let strings = ["foo bar", "foo bar biz bang bazinga!", "foo bar biz bang"];

    let_gc_and_context!(gc, ctx);

    let string0 = GCString::new(&ctx, strings[0]);
    let string1 = GCString::new(&ctx, strings[1]);
    let string2 = GCString::new(&ctx, strings[2]);

    assert_eq!(string0.raw().get_inline(), Some(strings[0]));
    assert_eq!(string1.raw().get_inline(), None);
    assert_eq!(string2.raw().get_inline(), None);

    assert_eq!(string0.read(), strings[0]);
    assert_eq!(string1.read(), strings[1]);
    assert_eq!(string2.read(), strings[2]);
}
