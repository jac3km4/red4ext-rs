use std::marker::PhantomData;

use const_combine::bounded::const_combine as combine;

use crate::NativeRepr;
use crate::raw::root::RED4ext as red;

// temporary module, we should split it up into separate files

#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct NodeRef(red::NodeRef);

unsafe impl NativeRepr for NodeRef {
    const NAME: &'static str = "worldGlobalNodeRef";
}

#[derive(Debug)]
#[repr(transparent)]
pub struct DataBuffer(red::DataBuffer);

#[derive(Debug)]
#[repr(transparent)]
pub struct DeferredDataBuffer(red::DeferredDataBuffer);

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct SharedDataBuffer(red::SharedDataBuffer);

#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct DateTime(red::CDateTime);

#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct Guid(red::CGUID);

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct EditorObjectId(red::EditorObjectID);

#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct MessageResourcePath(red::MessageResourcePath);

#[repr(transparent)]
pub struct Variant(red::Variant);

#[derive(Debug)]
#[repr(transparent)]
pub struct ResourceRef<T>(red::ResourceReference<T>);

#[derive(Debug)]
#[repr(transparent)]
pub struct Curve<T>(red::CurveData, PhantomData<T>);

#[derive(Debug)]
#[repr(transparent)]
pub struct MultiChannelCurve<T>([u8; 56], PhantomData<T>);

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
