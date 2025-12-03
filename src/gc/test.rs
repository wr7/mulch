use super::gcspace::GCSpace;

#[test]
fn gcspace_string_test() {
    let strings = ["foo bar", "foo bar biz bang bazinga!", "foo bar biz bang"];
    let mut gcspace = GCSpace::new();

    let string0 = gcspace.alloc_string(strings[0]);
    let string1 = gcspace.alloc_string(strings[1]);
    let string2 = gcspace.alloc_string(strings[2]);

    assert_eq!(string0.get_inline(), Some(strings[0]));
    assert_eq!(string1.get_inline(), None);
    assert_eq!(string2.get_inline(), None);

    assert_eq!(unsafe { gcspace.get_string(&string0) }, strings[0]);
    assert_eq!(unsafe { gcspace.get_string(&string1) }, strings[1]);
    assert_eq!(unsafe { gcspace.get_string(&string2) }, strings[2]);
}
