//! A dynamically sized array.
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::{fmt, mem, ops, ptr, slice};

use super::IAllocator;
use crate::VoidPtr;
use crate::raw::root::RED4ext as red;

/// A dynamically sized array.
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
        unsafe { ptr::drop_in_place(elems) }
        self.0.size = 0;
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

fn get_allocator<T>(arr: &red::DynArray<T>) -> *mut IAllocator {
    if arr.capacity == 0 {
        &arr.entries as *const _ as *mut _
    } else {
        let end = unsafe { arr.entries.add(arr.capacity as _) } as usize;
        let aligned = end.next_multiple_of(mem::size_of::<usize>());
        aligned as _
    }
}
