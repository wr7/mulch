use crate::util::MultiPeekable;

#[test]
fn multi_peekable() {
    let nums = 0..10;
    let mut mp = MultiPeekable::<_, 3>::new(nums);

    assert_eq!(mp.peek_all(), &[0, 1, 2]);
    assert_eq!(mp.next(), Some(0));
    assert_eq!(mp.peek_all(), &[1, 2, 3]);
    assert_eq!(mp.peek(0), Some(&1));
    assert_eq!(mp.peek(1), Some(&2));
    assert_eq!(mp.peek(2), Some(&3));
    assert_eq!(mp.peek(3), None);
}
