use std::ptr::{addr_of, addr_of_mut};

/// Keeps track of whether objects are dropped. This will detect double-frees and leaks.
pub struct DropReg {
    pub register: Option<usize>,
    pub expected: usize,
}

impl DropReg {
    pub fn new(id: usize) -> Self {
        Self {
            register: None,
            expected: id,
        }
    }

    pub fn get<'a>(&'a mut self) -> DropTester<'a> {
        DropTester {
            reg: self,
            dropped: false,
        }
    }

    pub fn assert_dropped(&self) {
        let Some(reg) = self.register else {
            panic!("Object was not dropped")
        };

        if reg != self.expected {
            panic!("Undefined behavior triggered")
        }
    }
}

pub struct DropTester<'a> {
    pub reg: &'a mut DropReg,
    pub dropped: bool,
}

impl<'a> DropTester<'a> {
    pub fn assert(&self) {
        if unsafe { addr_of!(self.dropped).read_volatile() } {
            panic!("Use-after drop detected!")
        }
    }
}

impl<'a> Drop for DropTester<'a> {
    fn drop(&mut self) {
        if self.reg.register.is_some() {
            panic!("Double drop detected!")
        }

        self.reg.register = Some(self.reg.expected);
        unsafe { addr_of_mut!(self.dropped).write_volatile(true) };
    }
}
