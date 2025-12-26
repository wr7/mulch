use std::cell::UnsafeCell;

use crate::{
    eval,
    gc::{GCPtr, GarbageCollector},
    util::IVec,
};

#[cfg(test)]
mod test;

pub struct GCRoot<'r> {
    prev: Option<&'r GCRoot<'r>>,
    data: UnsafeCell<IVec<1, eval::Value>>,
}

impl<'r> GCRoot<'r> {
    pub fn new_empty() -> Self {
        Self {
            prev: None,
            data: UnsafeCell::new(IVec::new()),
        }
    }

    pub fn new<'b>(&'b self) -> GCRoot<'b>
    where
        'b: 'r,
    {
        Self {
            prev: Some(self),
            data: UnsafeCell::new(IVec::new()),
        }
    }

    pub fn get(&self, idx: usize) -> Option<eval::Value> {
        unsafe {
            self.data
                .get()
                .as_mut()
                .unwrap_unchecked()
                .get(idx)
                .copied()
        }
    }

    pub fn push(&mut self, value: eval::Value) {
        self.data.get_mut().push(value);
    }

    /// Returns a slice with all of the `Values` in the current level of the root.
    pub fn as_mut_slice(&mut self) -> &mut [eval::Value] {
        self.data.get_mut().as_mut_slice()
    }
}

type RootsRef<'r> = &'r GCRoot<'r>;

impl GarbageCollector {
    unsafe fn copy_roots<'r>(&mut self, root: RootsRef<'r>) {
        let mut root = root;

        loop {
            let root_data = unsafe { &mut *root.data.get() };

            for root in root_data.iter_mut() {
                *root = unsafe { root.gc_copy(self) };
            }

            let Some(prev_root) = root.prev else {
                return;
            };

            root = prev_root
        }
    }

    /// Does a garbage collection cycle.
    ///
    /// NOTE: All objects contained in `root` will be moved and all references inside of `root` will
    /// be updated. **Any other references will become invalid even if they point to an object that
    /// survives the garbage-collection cycle**.
    ///
    /// # Safety
    /// All `Value`s in `root` must point to valid, currently alive objects in `from-space` of the
    /// current `GarbageCollector`.
    pub unsafe fn force_collect<'r>(&mut self, root: RootsRef<'r>) {
        unsafe {
            self.copy_roots(root);
        }

        std::mem::swap(&mut self.from_space, &mut self.to_space);
    }
}
