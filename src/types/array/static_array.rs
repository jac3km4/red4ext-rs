use const_combine::bounded::const_combine as combine;

use crate::NativeRepr;

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
    #[inline]
    pub fn entries(&self) -> &[T] {
        &self.entries[..self.size as usize]
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.size
    }
}
