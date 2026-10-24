use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::{fmt, mem, ops, ptr, slice};

use super::IAllocator;
use crate::VoidPtr;
use crate::raw::root::RED4ext as red;

mod static_array;
pub use static_array::StaticArray;

/// Owns an SDK [DynArray](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/DynArray.hpp).
/// Allocation and cleanup require the engine; drop destroys elements and releases storage.
#[repr(transparent)]
pub struct RedArray<T>(red::DynArray<T>);

impl<T> RedArray<T> {
    /// Creates a new empty [`RedArray`].
    #[inline]
    pub const fn new() -> Self {
        Self(red::DynArray {
            entries: ptr::null_mut(),
            size: 0,
            capacity: 0,
            _phantom_0: PhantomData,
        })
    }

    /// Creates a new empty [`RedArray`] with the specified capacity.
    #[inline]
    pub fn with_capacity(capacity: u32) -> Self {
        let mut this = Self::new();
        this.realloc(capacity);
        this
    }

    /// Returns the number of elements in the array.
    #[inline]
    pub fn len(&self) -> u32 {
        self.0.size
    }

    /// Returns `true` if the array contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements the array can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.0.capacity
    }

    /// Clears the array, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        let elems: *mut [T] = &mut **self;
        self.0.size = 0;
        unsafe { ptr::drop_in_place(elems) }
    }

    /// Adds an element to the end of the array.
    #[inline]
    pub fn push(&mut self, value: T) {
        let len = self.len();
        self.reserve(1);
        unsafe {
            ptr::write(self.0.entries.add(len as usize), value);
        }
        self.0.size = len + 1;
    }

    /// Reserve capacity for at least `additional` more elements to be inserted.
    pub fn reserve(&mut self, additional: u32) {
        let expected = self.len() + additional;
        if expected <= self.capacity() {
            return;
        }
        self.realloc(expected.max(self.capacity() + self.capacity() / 2));
    }

    /// Returns an iterator over the elements of the array.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.into_iter()
    }

    fn realloc(&mut self, cap: u32) {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>().max(8);
        unsafe {
            let realloc = crate::fn_from_hash!(
                DynArray_Realloc,
                unsafe extern "C" fn(VoidPtr, u32, u32, u32, usize)
            );
            realloc(self as *mut _ as VoidPtr, cap, size as u32, align as u32, 0);
        };
    }
}

impl<T: fmt::Debug> fmt::Debug for RedArray<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T> Default for RedArray<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> ops::Deref for RedArray<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        if !self.0.entries.is_null() {
            unsafe { slice::from_raw_parts(self.0.entries, self.len() as _) }
        } else {
            Default::default()
        }
    }
}

impl<T> ops::DerefMut for RedArray<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        if !self.0.entries.is_null() {
            unsafe { slice::from_raw_parts_mut(self.0.entries, self.len() as _) }
        } else {
            Default::default()
        }
    }
}

impl<T> AsRef<[T]> for RedArray<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T> AsMut<[T]> for RedArray<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'a, T> IntoIterator for &'a RedArray<T> {
    type IntoIter = slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter(self)
    }
}

impl<T> IntoIterator for RedArray<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let me = mem::ManuallyDrop::new(self);
        IntoIter {
            array: red::DynArray {
                entries: me.0.entries,
                size: me.len(),
                capacity: me.capacity(),
                ..Default::default()
            },
            ptr: me.0.entries,
            end: unsafe { me.0.entries.add(me.len() as _) },
        }
    }
}

impl<T> FromIterator<T> for RedArray<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array = Self::new();
        array.extend(iter);
        array
    }
}

impl<T> Extend<T> for RedArray<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let (lower, _) = iter.size_hint();
        self.reserve(lower as u32);
        for item in iter {
            self.push(item);
        }
    }
}

impl<T> Drop for RedArray<T> {
    #[inline]
    fn drop(&mut self) {
        if self.capacity() == 0 {
            return;
        }
        let elems: *mut [T] = &mut **self;
        unsafe {
            ptr::drop_in_place(elems);
            (*get_allocator(&self.0)).free(self.0.entries);
        };
    }
}

#[derive(Debug)]
pub struct IntoIter<T> {
    array: red::DynArray<T>,
    ptr: *mut T,
    end: *mut T,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {
            None
        } else {
            let old = self.ptr;
            self.ptr = unsafe { old.add(1) };
            Some(unsafe { ptr::read(old) })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = unsafe { self.end.offset_from(self.ptr) } as usize;
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.ptr == self.end {
            None
        } else {
            self.end = unsafe { self.end.sub(1) };
            Some(unsafe { ptr::read(self.end) })
        }
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> FusedIterator for IntoIter<T> {}

impl<T> Drop for IntoIter<T> {
    #[inline]
    fn drop(&mut self) {
        if self.array.capacity == 0 {
            return;
        }
        unsafe {
            let elems = slice::from_raw_parts_mut(self.ptr, self.len());
            ptr::drop_in_place(elems);
            (*get_allocator(&self.array)).free(self.array.entries);
        };
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use super::*;

    #[test]
    fn clear_empties_before_panicking_drop() {
        struct PanickingDrop(Rc<Cell<usize>>);
        impl Drop for PanickingDrop {
            fn drop(&mut self) {
                let drops = self.0.get();
                self.0.set(drops + 1);
                if drops == 0 {
                    panic!("synthetic drop failure");
                }
            }
        }

        let drops = Rc::new(Cell::new(0));
        let mut storage = mem::ManuallyDrop::new([
            PanickingDrop(Rc::clone(&drops)),
            PanickingDrop(Rc::clone(&drops)),
        ]);
        // Borrow stack storage without transferring its allocation to the engine.
        let mut array = RedArray(red::DynArray {
            entries: storage.as_mut_ptr(),
            size: 2,
            capacity: 2,
            ..Default::default()
        });
        assert!(catch_unwind(AssertUnwindSafe(|| array.clear())).is_err());
        assert!(array.is_empty());
        assert_eq!(drops.get(), 2);
        array.clear();
        // Detach the borrowed allocation before testing array drop.
        array.0.entries = ptr::null_mut();
        array.0.capacity = 0;
        drop(array);
        assert_eq!(drops.get(), 2);
    }
}

fn get_allocator<T>(arr: &red::DynArray<T>) -> *mut IAllocator {
    if arr.capacity == 0 {
        &arr.entries as *const _ as *mut _
    } else {
        let end = unsafe { arr.entries.add(arr.capacity as _) } as usize;
        let aligned = end.next_multiple_of(mem::size_of::<usize>());
        aligned as _
    }
}
