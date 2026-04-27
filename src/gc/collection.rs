use crate::gc::GarbageCollector;

#[cfg(test)]
mod test;

mod gcvalue;

impl GarbageCollector {
    unsafe fn copy_roots(&mut self) {
        let num_roots = self.roots.len();

        for i in 0..num_roots {
            let old_entry = unsafe { self.roots.get_unchecked(i) };

            let mut new_entry = old_entry;
            new_entry.data_ptr = unsafe { (old_entry.copy_fn)(old_entry.data_ptr, self) };

            self.roots.set(i, new_entry);
        }

        assert_eq!(self.roots.len(), num_roots);
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
    pub unsafe fn collect<'r>(&mut self) {
        if self.from_space.len() < self.from_space.capacity() * 15 / 16 {
            return;
        }

        unsafe { self.force_collect() };

        if self.from_space.len() < self.from_space.capacity() * 12 / 16 {
            return;
        }

        self.to_space
            .expand_capacity_to_exact(self.from_space.capacity() * 2);
        self.from_space
            .expand_capacity_to_exact(self.from_space.capacity() * 2);
    }

    /// Forcefully does a garbage collection cycle.
    ///
    /// See documentation of [`GarbageCollector::collect`] for safety and any other information.
    #[allow(clippy::missing_safety_doc)]
    #[cold]
    pub unsafe fn force_collect<'r>(&mut self) {
        unsafe {
            self.copy_roots();
        }

        std::mem::swap(&mut self.from_space, &mut self.to_space);
    }
}
