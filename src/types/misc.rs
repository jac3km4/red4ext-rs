use std::marker::PhantomData;
use std::{mem, ptr};

use const_combine::bounded::const_combine as combine;

use crate::raw::root::RED4ext as red;
use crate::types::{CName, Type};
use crate::{FromRepr, IntoRepr, NativeRepr, RttiSystem};

// temporary module, we should split it up into separate files

#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

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

impl Variant {
    pub fn new<A: IntoRepr>(val: A) -> Option<Self> {
        let mut this = Self::default();
        let rtti = RttiSystem::get();
        let typ = rtti.get_type(CName::new(A::Repr::NAME))?;
        // Variant owns the repr, so we need to prevent the compiler from dropping it.
        let mut repr = mem::ManuallyDrop::new(val.into_repr());
        if !unsafe { this.0.Fill(typ.as_raw(), ptr::from_mut(&mut repr).cast()) } {
            return None;
        }
        Some(this)
    }

    #[inline]
    pub fn type_(&self) -> Option<&Type> {
        unsafe { Type::from_raw(self.0.GetType()) }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        let size = self.type_()?.size() as usize;
        let data_ptr = unsafe { self.0.GetDataPtr() } as *const u8;
        if data_ptr.is_null() {
            None
        } else {
            Some(unsafe { std::slice::from_raw_parts(data_ptr, size) })
        }
    }

    pub fn try_clone<A>(&self) -> Option<A>
    where
        A: FromRepr,
        A::Repr: Clone,
    {
        let repr = unsafe { self.try_access::<A>()?.as_ref() }?;
        Some(A::from_repr(repr.clone()))
    }

    pub fn try_take<A: FromRepr>(&mut self) -> Option<A> {
        let repr = self.try_access::<A>()?;
        let value = unsafe { repr.read() };
        // We use ptr::write to prevent the compiler from dropping the Variant.
        // Otherwise we would have a double free of the repr.
        unsafe { ptr::write(self, Self::default()) }
        Some(A::from_repr(value))
    }

    fn try_access<A: FromRepr>(&self) -> Option<*const A::Repr> {
        if self.type_()?.name() == CName::new(A::Repr::NAME) {
            Some(unsafe { self.0.GetDataPtr() }.cast::<A::Repr>())
        } else {
            None
        }
    }
}

impl Default for Variant {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Clone for Variant {
    #[inline]
    fn clone(&self) -> Self {
        Self(unsafe { red::Variant::new4(&self.0) })
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe { self.0.Free() }
    }
}

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
