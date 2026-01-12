mod collection;
mod gcspace;
mod primitives;

pub mod util;
pub use collection::{GCRoot, GCValue, GCValueEnum};
pub use gcspace::GCPtr;
pub use primitives::*;

#[cfg(test)]
mod test;

pub struct GarbageCollector {
    from_space: GCSpace,
    to_space: GCSpace,
}

pub struct GCSpace {
    data: *mut u8,
    /// Currently occupied space (in blocks)
    len: usize,
    /// Capacity (in blocks)
    capacity: usize,
}

impl GarbageCollector {
    const BLOCK_SIZE: usize = crate::util::ceil_power_two(crate::util::max!(
        std::mem::align_of::<crate::parser_old::Expression>(),
        std::mem::align_of::<crate::eval::MValue>(),
        std::mem::align_of::<usize>(),
        std::mem::size_of::<usize>()
    ));

    pub fn new() -> Self {
        GarbageCollector {
            from_space: GCSpace::new(),
            to_space: GCSpace::new(),
        }
    }
}
