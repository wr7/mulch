use crate::gc::safety::GCCtx;

/// A bundle of garbage-collected arguments. These are used for calling functions that
/// can trigger GC cycles but also take in unmanaged garbage-collected data.
///
/// This can only be created using the [`gc_args!()`](super::gc_args) macro.
pub struct GCArgs<'gc, 'ctx, B> {
    context: &'ctx mut GCCtx<'gc>,
    inner_tuple: B,
}

impl<'gc, 'ctx, B> GCArgs<'gc, 'ctx, B> {
    /// # Safety
    /// - `raw` must be a tuple of raw garbage collected datatypes.
    /// - All values in `raw` should be initialized and valid until another GC cycle is triggered.
    pub unsafe fn new(context: &'ctx mut GCCtx<'gc>, raw: B) -> Self {
        Self {
            context,
            inner_tuple: raw,
        }
    }

    /// Splits `GCArgs` into a mutable context reference and its raw GC pointers.
    pub fn split(self) -> (&'ctx mut GCCtx<'gc>, B) {
        (self.context, self.inner_tuple)
    }
}
