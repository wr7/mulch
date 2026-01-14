use std::{
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
};

#[cfg(test)]
mod test;

/// A dynamically-sized array that can store `N` elements on the stack.
///
/// If the length exceeds `N`, its elements are stored on the heap.
pub struct IVec<const N: usize, T> {
    len: usize, // The first MSB indicates that it is stored inline
    data: IVecData<N, T>,
}

impl<const N: usize, T> IVec<N, T> {
    fn data<'a>(&'a self) -> IVecDataEnum<'a, N, T> {
        if self.is_inline() {
            IVecDataEnum::Inline(unsafe { &self.data.inline })
        } else {
            IVecDataEnum::Heap(unsafe { &self.data.heap })
        }
    }

    fn data_mut<'a>(&'a mut self) -> IVecDataMutEnum<'a, N, T> {
        if self.is_inline() {
            IVecDataMutEnum::Inline(unsafe { &mut self.data.inline })
        } else {
            IVecDataMutEnum::Heap(unsafe { &mut self.data.heap })
        }
    }

    pub fn len(&self) -> usize {
        self.len & (!0usize >> 1)
    }

    pub fn is_inline(&self) -> bool {
        self.len & 1usize.rotate_right(1) != 0
    }

    pub fn new() -> Self {
        unsafe {
            Self {
                len: 1usize.rotate_right(1),
                data: IVecData {
                    inline: ManuallyDrop::new(MaybeUninit::uninit().assume_init()), // cursed
                },
            }
        }
    }

    pub fn as_slice(&self) -> &[T] {
        match self.data() {
            IVecDataEnum::Inline(data) => unsafe {
                std::slice::from_raw_parts(data.as_ptr().cast::<T>(), self.len())
            },
            IVecDataEnum::Heap(data) => unsafe { std::slice::from_raw_parts(data.ptr, self.len) },
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match self.data_mut() {
            IVecDataMutEnum::Inline(data) => unsafe {
                std::slice::from_raw_parts_mut(data.as_mut_ptr().cast::<T>(), self.len())
            },
            IVecDataMutEnum::Heap(data) => unsafe {
                std::slice::from_raw_parts_mut(data.ptr, self.len)
            },
        }
    }

    pub fn push(&mut self, val: T) {
        let len = self.len();

        match self.data_mut() {
            IVecDataMutEnum::Inline(data) => {
                if let Some(element) = data.get_mut(len) {
                    element.write(val);
                    self.len += 1;
                } else {
                    // We do not have space to store this inline. We must move everything onto the
                    // heap
                    let vec: Vec<T> = data
                        .iter()
                        .map(|v| unsafe { v.assume_init_read() })
                        .chain(std::iter::once(val))
                        .collect();

                    let mut buf = Self::from(vec);

                    std::mem::swap(self, &mut buf);
                    std::mem::forget(buf);
                }
            }
            IVecDataMutEnum::Heap(data) => {
                let mut vec = unsafe { Vec::from_raw_parts(data.ptr, len, data.capacity) };

                vec.push(val);

                let mut buf = Self::from(vec);

                std::mem::swap(self, &mut buf);
                std::mem::forget(buf);
            }
        }
    }
}

struct IVecHeapData<const N: usize, T> {
    capacity: usize,
    ptr: *mut T,
}

impl<const N: usize, T> Clone for IVecHeapData<N, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: usize, T> Copy for IVecHeapData<N, T> {}

enum IVecDataEnum<'a, const N: usize, T> {
    Inline(&'a [MaybeUninit<T>; N]),
    Heap(&'a IVecHeapData<N, T>),
}

enum IVecDataMutEnum<'a, const N: usize, T> {
    Inline(&'a mut [MaybeUninit<T>; N]),
    Heap(&'a mut IVecHeapData<N, T>),
}

union IVecData<const N: usize, T> {
    inline: ManuallyDrop<[MaybeUninit<T>; N]>,
    heap: IVecHeapData<N, T>,
}

impl<const N: usize, T> AsRef<[T]> for IVec<N, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<const N: usize, T> AsMut<[T]> for IVec<N, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<const N: usize, T> Deref for IVec<N, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<const N: usize, T> DerefMut for IVec<N, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<const N: usize, T> Drop for IVec<N, T> {
    fn drop(&mut self) {
        if self.is_inline() {
            for i in 0..self.len() {
                unsafe { (*self.data.inline).get_mut(i).unwrap().assume_init_drop() };
            }
        } else {
            unsafe {
                std::vec::Vec::from_raw_parts(
                    self.data.heap.ptr,
                    self.len,
                    self.data.heap.capacity,
                );
            }
        }
    }
}

impl<const N: usize, T> From<Vec<T>> for IVec<N, T> {
    fn from(mut value: Vec<T>) -> Self {
        let len = value.len();
        let capacity = value.capacity();
        let ptr = value.as_mut_ptr();

        std::mem::forget(value);

        IVec {
            len,
            data: IVecData {
                heap: IVecHeapData { capacity, ptr },
            },
        }
    }
}
