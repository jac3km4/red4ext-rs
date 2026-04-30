use std::array::IntoIter;
use std::ops::{Deref, DerefMut};

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
    /// Returns the elements of the array as a slice.
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
    pub fn len(&self) -> u32 {
        self.size
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
}

impl<T, const N: usize> Deref for StaticArray<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        &self.entries[..self.size as usize]
    }
}

impl<T, const N: usize> DerefMut for StaticArray<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.entries[..self.size as usize]
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
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.deref().into_iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut StaticArray<T, N> {
    type IntoIter = std::slice::IterMut<'a, T>;
    type Item = &'a mut T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.deref_mut().into_iter()
    }
}

impl<T, const N: usize> IntoIterator for StaticArray<T, N> {
    type IntoIter = std::iter::Take<IntoIter<T, N>>;
    type Item = T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let size = self.size as usize;
        self.entries.into_iter().take(size)
    }
}
