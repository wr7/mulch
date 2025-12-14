use crate::gc::gcspace::GCSpace;

mod gcspace;

#[cfg(test)]
mod test;

pub struct GarbageCollector {
    from_space: GCSpace,
    to_space: GCSpace,
}

impl GarbageCollector {
    const BLOCK_SIZE: usize = crate::util::ceil_power_two(crate::util::max!(
        std::mem::align_of::<crate::parser::Expression>(),
        std::mem::align_of::<usize>(),
        std::mem::size_of::<usize>()
    ));
}
