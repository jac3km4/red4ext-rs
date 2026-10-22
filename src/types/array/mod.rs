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
            m_entries: ptr::null_mut(),
            m_size: 0,
            m_capacity: 0,
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
        self.0.m_size
    }

    /// Returns `true` if the array contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements the array can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.0.m_capacity
    }

    /// Clears the array, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        let elems: *mut [T] = &mut **self;
        self.0.m_size = 0;
        unsafe { ptr::drop_in_place(elems) }
    }

    /// Adds an element to the end of the array.
    #[inline]
    pub fn push(&mut self, value: T) {
        let len = self.len();
        self.reserve(1);
        unsafe {
            ptr::write(self.0.m_entries.add(len as usize), value);
        }
        self.0.m_size = len + 1;
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
        if self.0.m_capacity != 0 && !self.is_empty() {
            unsafe { slice::from_raw_parts(self.0.m_entries, self.len() as _) }
        } else {
            Default::default()
        }
    }
}

impl<T> ops::DerefMut for RedArray<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        if self.0.m_capacity != 0 && !self.is_empty() {
            unsafe { slice::from_raw_parts_mut(self.0.m_entries, self.len() as _) }
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
        // With zero capacity, entries stores an allocator vtable, not elements.
        let ptr = if me.is_empty() {
            ptr::NonNull::<T>::dangling().as_ptr()
        } else {
            me.0.m_entries
        };
        IntoIter {
            array: red::DynArray {
                m_entries: me.0.m_entries,
                m_size: me.len(),
                m_capacity: me.capacity(),
                ..Default::default()
            },
            ptr,
            remaining: me.len() as usize,
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
            (*get_allocator(&self.0)).free(self.0.m_entries);
        };
    }
}

#[derive(Debug)]
pub struct IntoIter<T> {
    array: red::DynArray<T>,
    ptr: *mut T,
    remaining: usize,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            let old = self.ptr;
            self.ptr = unsafe { old.add(1) };
            Some(unsafe { ptr::read(old) })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            Some(unsafe { ptr::read(self.ptr.add(self.remaining)) })
        }
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> FusedIterator for IntoIter<T> {}

impl<T> Drop for IntoIter<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let elems = ptr::slice_from_raw_parts_mut(self.ptr, self.remaining);
            self.remaining = 0;
            ptr::drop_in_place(elems);
            if self.array.m_capacity != 0 {
                (*get_allocator(&self.array)).free(self.array.m_entries);
            }
        };
    }
}

fn get_allocator<T>(arr: &red::DynArray<T>) -> *mut IAllocator {
    if arr.m_capacity == 0 {
        &arr.m_entries as *const _ as *mut _
    } else {
        let end = unsafe { arr.m_entries.add(arr.m_capacity as _) } as usize;
        let aligned = end.next_multiple_of(mem::size_of::<usize>());
        aligned as _
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use super::*;

    #[test]
    fn empty_views_ignore_allocator_vtable() {
        #[repr(align(256))]
        struct Aligned {
            _byte: u8,
        }
        let mut array = RedArray::<Aligned>::new();
        // A non-null, deliberately unaligned stand-in for an SDK allocator vtable.
        array.0.m_entries = ptr::without_provenance_mut(1);
        assert!(array.as_ref().is_empty());
        assert!(array.as_mut().is_empty());
        array.clear();
        let mut iter = array.into_iter();
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert!(iter.next().is_none());
        assert!(iter.next_back().is_none());
        drop(iter);
    }

    #[test]
    fn null_empty_iterator_is_fused() {
        let mut iter = RedArray::<u64>::new().into_iter();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn consuming_zsts_tracks_counts_and_drops_remainder() {
        thread_local! {
            static DROPS: Cell<usize> = const { Cell::new(0) };
        }
        struct Zst;
        impl Drop for Zst {
            fn drop(&mut self) {
                DROPS.with(|drops| drops.set(drops.get() + 1));
            }
        }
        assert_eq!(mem::size_of::<Zst>(), 0);
        DROPS.with(|drops| drops.set(0));
        // No allocation is needed for these synthetic ZST owners.
        let mut iter = IntoIter {
            array: red::DynArray::default(),
            ptr: ptr::NonNull::<Zst>::dangling().as_ptr(),
            remaining: 5,
        };
        assert_eq!(iter.len(), 5);
        drop(iter.next().unwrap());
        assert_eq!(iter.size_hint(), (4, Some(4)));
        drop(iter.next_back().unwrap());
        assert_eq!(iter.len(), 3);
        drop(iter);
        DROPS.with(|drops| assert_eq!(drops.get(), 5));

        let mut iter = IntoIter {
            array: red::DynArray::default(),
            ptr: ptr::NonNull::<Zst>::dangling().as_ptr(),
            remaining: 2,
        };
        drop(iter.next_back().unwrap());
        drop(iter.next().unwrap());
        assert!(iter.next().is_none());
        assert!(iter.next_back().is_none());
        assert_eq!(iter.len(), 0);
        drop(iter);
        DROPS.with(|drops| assert_eq!(drops.get(), 7));
    }

    #[test]
    fn consuming_non_zsts_supports_mixed_ends() {
        let mut storage = [10, 20, 30, 40];
        let mut iter = IntoIter {
            array: red::DynArray::default(),
            ptr: storage.as_mut_ptr(),
            remaining: storage.len(),
        };
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next_back(), Some(40));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.collect::<Vec<_>>(), [20, 30]);
    }

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
            m_entries: storage.as_mut_ptr(),
            m_size: 2,
            m_capacity: 2,
            ..Default::default()
        });
        assert!(catch_unwind(AssertUnwindSafe(|| array.clear())).is_err());
        assert!(array.is_empty());
        assert_eq!(drops.get(), 2);
        array.clear();
        // Detach the borrowed allocation before testing array drop.
        array.0.m_entries = ptr::null_mut();
        array.0.m_capacity = 0;
        drop(array);
        assert_eq!(drops.get(), 2);
    }
}
