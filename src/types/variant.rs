use std::{mem, ptr};

use crate::raw::root::RED4ext as red;
use crate::types::{CName, Type};
use crate::{FromRepr, IntoRepr, NativeRepr, RttiSystem};

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

    pub fn try_take<A: FromRepr>(&mut self) -> Option<A> {
        let repr = self.try_access::<A::Repr>()?;
        let value = unsafe { ptr::read(repr) };
        // We use ptr::write to prevent the compiler from dropping the Variant.
        // Otherwise we would have a double free of the repr.
        unsafe { ptr::write(self, Self::default()) }
        Some(A::from_repr(value))
    }

    pub fn try_access<A: NativeRepr>(&self) -> Option<&A> {
        if self.type_()?.name() == CName::new(A::NAME) {
            unsafe { self.0.GetDataPtr().cast::<A>().as_ref() }
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
