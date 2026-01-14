use std::cell::UnsafeCell;

use crate::{
    eval,
    gc::{GCPtr, GarbageCollector},
    util::IVec,
};

#[cfg(test)]
mod test;

mod gcvalue;

pub use gcvalue::GCValue;
pub use gcvalue::GCValueEnum;

pub struct GCRoot<'r> {
    prev: Option<&'r GCRoot<'r>>,
    data: UnsafeCell<IVec<1, GCValue>>,
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

    pub fn get_value(&self, idx: usize) -> Option<eval::MValue> {
        self.get(idx).and_then(|v| v.try_into().ok())
    }

    pub fn get(&self, idx: usize) -> Option<GCValue> {
        unsafe {
            self.data
                .get()
                .as_mut()
                .unwrap_unchecked()
                .get(idx)
                .copied()
        }
    }

    pub fn push(&mut self, value: impl Into<GCValue>) {
        self.data.get_mut().push(value.into());
    }

    /// Returns a slice of all `MValue`s in the root
    ///
    /// # Safety
    /// - The current root must not contain any non-`MValue` `GCValue`s
    pub unsafe fn as_mut_mvalue_slice(&mut self) -> &mut [eval::MValue] {
        const {
            // These are needed to ensure that `GCValue` can be transmuted into `MValue`
            assert!(std::mem::size_of::<eval::MValue>() == std::mem::size_of::<GCValue>());
            assert!(std::mem::align_of::<eval::MValue>() <= std::mem::align_of::<GCValue>());
        }

        unsafe {
            crate::util::transmute_mut_slice::<GCValue, eval::MValue>(
                self.data.get_mut().as_mut_slice(),
            )
        }
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

    /// Does a garbage collection cycle if it is deemed neccessary.
    ///
    /// NOTE: All objects contained in `root` will be moved and all references inside of `root` will
    /// be updated. **Any other references will become invalid even if they point to an object that
    /// survives the garbage-collection cycle**.
    ///
    /// # Safety
    /// All `Value`s in `root` must point to valid, currently alive objects in `from-space` of the
    /// current `GarbageCollector`.
    pub unsafe fn collect<'r>(&mut self, root: RootsRef<'r>) {
        if self.from_space.len() < self.from_space.capacity() * 15 / 16 {
            return;
        }

        unsafe { self.force_collect(root) };

        if self.from_space.len() < self.from_space.capacity() * 12 / 16 {
            return;
        }

        self.to_space.expand_exact(self.from_space.capacity() * 2);
        self.from_space.expand_exact(self.from_space.capacity() * 2);
    }

    /// Forcefully does a garbage collection cycle.
    ///
    /// See documentation of [`GarbageCollector::collect`] for safety and any other information.
    #[allow(clippy::missing_safety_doc)]
    #[cold]
    pub unsafe fn force_collect<'r>(&mut self, root: RootsRef<'r>) {
        unsafe {
            self.copy_roots(root);
        }

        std::mem::swap(&mut self.from_space, &mut self.to_space);
    }
}
