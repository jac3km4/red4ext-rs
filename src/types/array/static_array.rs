use const_combine::bounded::const_combine as combine;

use crate::NativeRepr;

/// Inline fixed-capacity array with a logical element count.
///
/// Conversion from `[T; N]` fills the array. [`NativeRepr`] names are supported
/// only for capacities 1 through 8.
///
/// SDK: [array RTTI](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
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
    /// Returns the elements within the logical size.
    #[inline]
    pub fn entries(&self) -> &[T] {
        &self.entries[..self.size as usize]
    }

    /// Returns the logical number of elements, not the capacity `N`.
    #[inline]
    pub fn size(&self) -> u32 {
        self.size
    }
}
