use crate::util::{
    IVec,
    ivec::test::util::{DropReg, DropTester},
};

mod util;

#[test]
/// Ensures that heap-allocated `IVec`s don't leak memory and don't double-free memory.
fn ivec_drop_heap() {
    let mut registers: Vec<DropReg> = (1..=5).map(DropReg::new).collect();

    let mut ivec: IVec<2, DropTester> = IVec::new();

    for reg in registers.iter_mut() {
        ivec.push(reg.get());
    }

    for t in ivec.iter() {
        t.assert();
    }

    assert!(!ivec.is_inline());
    std::mem::drop(ivec);

    for r in registers {
        r.assert_dropped();
    }
}

#[test]
/// Ensures that inline `IVec`s don't leak memory and don't double-free memory.
fn ivec_drop_stack() {
    drop(IVec::<0, Box<usize>>::new());
    drop(IVec::<1, Box<usize>>::new());

    let mut registers: Vec<DropReg> = (1..=2).map(DropReg::new).collect();

    let mut ivec: IVec<3, DropTester> = IVec::new();

    for reg in registers.iter_mut() {
        ivec.push(reg.get());
    }

    for t in ivec.iter() {
        t.assert();
    }

    assert!(ivec.is_inline());
    std::mem::drop(ivec);

    for r in registers {
        r.assert_dropped();
    }
}

#[test]
fn ivec_1() {
    let mut v: IVec<2, char> = IVec::new();
    assert_eq!(*v, []);
    assert!(v.is_inline());

    v.push('a');
    assert_eq!(*v, ['a']);
    assert!(v.is_inline());

    v.push('b');
    assert_eq!(*v, ['a', 'b']);
    assert!(v.is_inline());

    v.push('c');
    assert_eq!(*v, ['a', 'b', 'c']);
    assert!(!v.is_inline());

    v.push('d');
    assert_eq!(*v, ['a', 'b', 'c', 'd']);
    assert!(!v.is_inline());
}

#[test]
fn ivec_2() {
    let mut v: IVec<0, char> = IVec::new();
    assert_eq!(*v, []);
    assert!(v.is_inline());

    v.push('a');
    assert_eq!(*v, ['a']);
    assert!(!v.is_inline());

    v.push('b');
    assert_eq!(*v, ['a', 'b']);
    assert!(!v.is_inline());
}
