use std::marker::PhantomData;

use crate::raw::root::RED4ext as red;

// temporary module, we should split it up into separate files

#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

#[derive(Debug)]
#[repr(transparent)]
pub struct NodeRef(red::NodeRef);

#[derive(Debug)]
#[repr(transparent)]
pub struct DataBuffer(red::DataBuffer);

#[derive(Debug)]
#[repr(transparent)]
pub struct DeferredDataBuffer(red::DeferredDataBuffer);

#[derive(Debug)]
#[repr(transparent)]
pub struct SharedDataBuffer(red::SharedDataBuffer);

#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct DateTime(red::CDateTime);

#[derive(Debug, Default, Copy, Clone)]
#[repr(transparent)]
pub struct Guid(red::CGUID);

#[derive(Debug)]
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
