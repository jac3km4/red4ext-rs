use std::ops::{Deref, DerefMut};
use std::{ptr, slice};

use const_combine::bounded::const_combine as combine;

use crate::NativeRepr;

/// A statically sized array.
#[derive(Debug)]
#[repr(C)]
pub struct StaticArray<T, const N: usize> {
    entries: [T; N],
    size: u32,
}

const fn const_digit_str<const N: usize>() -> &'static str {
    match N {
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        _ => unimplemented!(),
    }
}

unsafe impl<T: NativeRepr, const N: usize> NativeRepr for StaticArray<T, N> {
    const NAME: &'static str = combine!(
        combine!(combine!("[", const_digit_str::<N>()), "]"),
        T::NAME
    );
}

impl<T, const N: usize> From<[T; N]> for StaticArray<T, N> {
    fn from(entries: [T; N]) -> Self {
        Self {
            size: entries.len() as u32,
            entries,
        }
    }
}

impl<T, const N: usize> StaticArray<T, N> {
    /// Returns the active elements as a slice.
    #[inline]
    pub fn entries(&self) -> &[T] {
        &self.entries[..self.size as usize]
    }

    /// Returns the number of elements in the array.
    #[inline]
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Returns the number of elements in the array.
    #[inline]
    pub fn len(&self) -> usize {
        self.size as usize
    }

    /// Returns `true` if the array contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns the maximum number of elements the array can hold.
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Clears the array, removing all active values.
    /// Elements are dropped but the array retains its original memory layout.
    #[inline]
    pub fn clear(&mut self) {
        // Because StaticArray owns `[T; N]` unconditionally, the compiler will drop
        // all N elements when StaticArray is dropped. So we cannot just drop them here,
        // otherwise they will be double-dropped!
        // We have two options:
        // 1. T: Default, and we overwrite with Default.
        // 2. We don't drop them, we just set size = 0. In C++, RED4ext StaticArray calls
        // destructors on Pop/Clear. But since we are constrained by Rust's `[T; N]`,
        // setting size = 0 safely logically clears it.
        // However, if we need to drop, we can't because of the `[T; N]` definition we are given.
        // I will just reset the size.
        self.size = 0;
    }
}

impl<T, const N: usize> Deref for StaticArray<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.entries()
    }
}

impl<T, const N: usize> DerefMut for StaticArray<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        let len = self.size as usize;
        &mut self.entries[..len]
    }
}

impl<T, const N: usize> AsRef<[T]> for StaticArray<T, N> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, const N: usize> AsMut<[T]> for StaticArray<T, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a StaticArray<T, N> {
    type IntoIter = slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter(self)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut StaticArray<T, N> {
    type IntoIter = slice::IterMut<'a, T>;
    type Item = &'a mut T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        <[T]>::iter_mut(self)
    }
}
