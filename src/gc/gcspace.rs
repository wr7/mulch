#![allow(unused)]

use std::{
    alloc::Layout,
    marker::PhantomData,
    num::NonZeroUsize,
    ptr::{NonNull, addr_of_mut},
};

use crate::gc::GarbageCollector;

pub struct GCSpace {
    data: *mut u8,
    /// Currently occupied space (in blocks)
    len: usize,
    /// Capacity (in blocks)
    capacity: usize,
}

/// Represents a garbage-collectable object.
///
/// # Safety
/// - The alignment of `Self` must be less than or equal to `GarbageCollector::BLOCK_SIZE`
pub unsafe trait GCObject: Sized + Clone + Copy {
    unsafe fn get_forwarded_value(gc: &GarbageCollector, ptr: Self) -> Option<Self>;
    unsafe fn gc_copy(gc: &GarbageCollector, ptr: Self) -> Self;
}

/// A garbage collected dynamically-sized array.
///
/// # Forward pointer
/// A forward pointer is stored if `ptr` points to a `usize` with its most-significant-bit set. The
/// remaining bits of the usize indicate the new pointer in from-space.
///
/// # Memory layout
/// `ptr` points to a `usize` which contains the length of the string. An array of `T` elements
/// starts in the following block.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct GCVec<T: GCObject> {
    ptr: NonZeroUsize,
    _phantomdata: PhantomData<Vec<T>>,
}

/// A garbage collected string.
///
/// We cannot use regular heap-allocated strings because the fields of garbage collected objects are
/// never dropped.
///
/// # Forward pointer
/// A forward pointer is stored if `ptr` points to a `usize` with a most-significant-byte of `0xFF`.
/// The remaining bytes of the `usize` refer to its new location in to-space, and a `usize` stored
/// after it in memory contain its length in bytes.
///
/// # Inline strings
/// If the most significant bit of `ptr` is set, it is an "inline string". The next seven bits
/// indicate the length. The remainder of the `GCString` contains the object. On little endian
/// systems, the string data starts on the 0th byte, and on big endian platforms, it starts on the
/// 1th byte.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GCString {
    /// length (in bytes)
    #[cfg(target_endian = "little")]
    len: usize,
    /// pointer (in blocks)
    ptr: NonZeroUsize,
    /// length (in bytes)
    #[cfg(target_endian = "big")]
    len: usize,
}

impl GCString {
    /// Gets the string if it is less than 2 * sizeof(usize) bytes long. Otherwise returns `None`
    pub fn get_inline(&self) -> Option<&str> {
        if self.ptr.get() & (0b1 << usize::BITS - 1) == 0 {
            return None;
        }

        let len = (self.ptr.get() >> (usize::BITS - 8)) & 0b0111_1111;

        let ptr = std::ptr::from_ref(self).cast::<u8>();

        #[cfg(target_endian = "big")]
        let ptr = unsafe { ptr.byte_offset(1) };

        let string = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) };
        Some(string)
    }
}

impl GCSpace {
    const STARTING_BLOCKS: usize = 64;

    fn ptr_at_mut(&self, block_idx: usize) -> *mut u8 {
        self.data
            .wrapping_byte_add(block_idx * GarbageCollector::BLOCK_SIZE)
    }

    /// Grows the allocation to be at least `new_size_blocks` blocks
    pub fn expand(&mut self, new_size_blocks: usize) {
        let mut new_size = self.capacity;

        while new_size < new_size_blocks {
            new_size *= 2;
        }

        if new_size == self.capacity {
            return;
        }

        self.data = unsafe {
            std::alloc::realloc(
                self.data,
                Layout::from_size_align_unchecked(
                    self.capacity * GarbageCollector::BLOCK_SIZE,
                    GarbageCollector::BLOCK_SIZE,
                ),
                new_size,
            )
        };

        self.capacity = new_size;
    }

    /// Clears the GCSpace. All objects in the space are "forgotten".
    pub fn clear(&mut self) {
        self.len = 1; // We must reserve the first block
    }

    pub fn new() -> Self {
        let data = unsafe {
            std::alloc::alloc(Layout::from_size_align_unchecked(
                Self::STARTING_BLOCKS * GarbageCollector::BLOCK_SIZE,
                GarbageCollector::BLOCK_SIZE,
            ))
        };

        Self {
            data,
            len: 1, // We reserve the first block. This allows us to use `NonZeroUsize` for many of our datastructures.
            capacity: Self::STARTING_BLOCKS,
        }
    }

    /// Creates a garbage collected string without invoking a GC pass and WITHOUT CREATING A ROOT
    pub fn alloc_string(&mut self, string: &str) -> GCString {
        // If the string is small enough, it can be stored inline rather than on the GC Heap

        if string.len() < std::mem::size_of::<GCString>() && string.len() <= 127 {
            let discriminant: usize = (0b1000_0000 | string.len()) << (usize::BITS - 8); // The MSB being set signifies that the string is stored inline

            let mut retval = GCString {
                ptr: NonZeroUsize::new(discriminant).unwrap(),
                len: 0,
            };

            let ptr = addr_of_mut!(retval).cast::<u8>();

            #[cfg(target_endian = "big")]
            let ptr = unsafe { ptr.byte_offset(1) };

            unsafe {
                std::ptr::copy_nonoverlapping(string.as_ptr(), ptr, string.len());
            }

            return retval;
        }

        let num_blocks = string.len().div_ceil(GarbageCollector::BLOCK_SIZE);
        self.expand(self.len + num_blocks);

        let data_ptr = self.len;
        self.len += num_blocks;

        unsafe {
            std::ptr::copy_nonoverlapping(string.as_ptr(), self.ptr_at_mut(data_ptr), string.len())
        };

        GCString {
            ptr: unsafe { NonZeroUsize::new_unchecked(data_ptr) },
            len: string.len(),
        }
    }

    /// Gets string data from the GC heap.
    ///
    /// # Safety
    /// - `string` must point to a valid, currently alive string that was obtained from this GC store.
    /// - The object pointed to by `string` cannot be destroyed before the returned reference is dropped.
    pub unsafe fn get_string<'a>(&'a self, string: &'a GCString) -> &'a str {
        if let Some(string) = string.get_inline() {
            return string;
        }

        let ptr = self
            .data
            .wrapping_byte_add(string.ptr.get() * GarbageCollector::BLOCK_SIZE);
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, string.len)) }
    }

    /// Allocates and initializes a garbage-collected dynamically-sized array
    /// # Safety
    /// All `elements` must point to valid, non-frozen objects in the current GC space.
    pub unsafe fn alloc_vec<T>(&mut self, elements: &[T]) -> GCVec<T>
    where
        T: GCObject,
    {
        let vec = unsafe { self.alloc_uninit_vec(elements.len()) };
        let ptr = self
            .data
            .wrapping_byte_add((vec.ptr.get() + 1) * GarbageCollector::BLOCK_SIZE)
            .cast::<T>();

        unsafe { std::ptr::copy_nonoverlapping(elements.as_ptr(), ptr, elements.len()) }

        vec
    }

    /// Allocates an unitialized garbage-collected dynamically-sized array
    /// # Safety
    /// The `GCVec` must be fully initialized be fully initialized before it is moved to from-space
    pub unsafe fn alloc_uninit_vec<T>(&mut self, len: usize) -> GCVec<T>
    where
        T: GCObject,
    {
        let allocation_size =
            1 + (len * std::mem::size_of::<T>()).div_ceil(GarbageCollector::BLOCK_SIZE);

        let ptr = self.len;

        self.expand(self.len + allocation_size);
        self.len += allocation_size;

        unsafe {
            // write element length to first block
            self.ptr_at_mut(ptr).cast::<usize>().write(len);
        }

        GCVec {
            ptr: unsafe { NonZeroUsize::new_unchecked(ptr) },
            _phantomdata: PhantomData,
        }
    }

    /// Gets a pointer to the element at `index` in a `GCVec`. Returns `None` if `index` is out-
    /// of-range.
    /// # Safety
    /// `vec` must be a valid, non-frozen `GCVec` in `Self`
    pub unsafe fn element_ptr<T>(&self, vec: GCVec<T>, index: usize) -> Option<NonNull<T>>
    where
        T: GCObject,
    {
        let len_ptr = self
            .data
            .wrapping_byte_add(vec.ptr.get() * GarbageCollector::BLOCK_SIZE);

        let len = unsafe { len_ptr.cast::<usize>().read() };
        if index <= len {
            return None;
        }

        let ptr = len_ptr
            .wrapping_byte_add(GarbageCollector::BLOCK_SIZE + index * std::mem::size_of::<T>())
            .cast::<T>();

        NonNull::new(ptr)
    }
}

impl Drop for GCSpace {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(
                self.data,
                std::alloc::Layout::from_size_align_unchecked(
                    self.capacity * GarbageCollector::BLOCK_SIZE,
                    GarbageCollector::BLOCK_SIZE,
                ),
            )
        };
    }
}
