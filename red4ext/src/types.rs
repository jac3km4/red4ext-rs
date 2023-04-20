use std::{mem, pin, ptr};

pub use ffi::IScriptable;
use red4ext_sys::ffi;
pub use red4ext_sys::interop::{CName, REDString, TweakDBID, Variant, VoidPtr};

use crate::conv::{FromRED, IntoRED};
use crate::rtti;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct REDArray<A> {
    entries: *mut A,
    cap: u32,
    size: u32,
}

impl<A> REDArray<A> {
    #[inline]
    pub fn as_slice(&self) -> &[A] {
        unsafe { std::slice::from_raw_parts(self.entries, self.size as usize) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [A] {
        unsafe { std::slice::from_raw_parts_mut(self.entries, self.size as usize) }
    }

    #[inline]
    pub fn with_capacity(count: usize) -> Self {
        let arr = REDArray::default();
        let ptr = VoidPtr(&arr as *const _ as *mut _);
        ffi::alloc_array(ptr, count as u32, mem::size_of::<A>() as u32);
        arr
    }

    pub fn from_sized_iter<I: ExactSizeIterator<Item = A>>(iter: I) -> Self {
        let len = iter.len();
        let mut arr: REDArray<A> = REDArray::with_capacity(len);
        for (i, elem) in iter.into_iter().enumerate() {
            unsafe { arr.entries.add(i).write(elem) }
        }
        arr.size = len as u32;
        arr
    }
}

impl<A> Default for REDArray<A> {
    #[inline]
    fn default() -> Self {
        Self {
            entries: ptr::null_mut(),
            cap: 0,
            size: 0,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Ref<A> {
    pub instance: *mut A,
    pub count: *mut RefCount,
}

impl<A> Ref<A> {
    #[inline]
    pub fn null() -> Self {
        Self::default()
    }
}

impl<A> Default for Ref<A> {
    #[inline]
    fn default() -> Self {
        Self {
            instance: ptr::null_mut(),
            count: ptr::null_mut(),
        }
    }
}

impl<A> Clone for Ref<A> {
    fn clone(&self) -> Self {
        Self {
            instance: self.instance,
            count: self.count,
        }
    }
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct RefCount {
    strong_refs: u32,
    weak_refs: u32,
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

pub trait VariantExt {
    fn new<A: IntoRED>(val: A) -> Self;
    fn try_get<A: FromRED>(&self) -> Option<A>;
}

impl VariantExt for Variant {
    fn new<A: IntoRED>(val: A) -> Self {
        let mut this = Self::default();
        let typ = rtti::get_type(CName::new(A::NAME));
        let repr = val.into_repr();
        unsafe {
            pin::Pin::new_unchecked(&mut this).fill(typ, VoidPtr(&repr as *const _ as *mut _));
        }
        this
    }

    fn try_get<A: FromRED>(&self) -> Option<A> {
        if rtti::get_type_name(self.get_type()) == CName::new(A::NAME) {
            let ptr = self.get_data_ptr().0 as *const <A as FromRED>::Repr;
            Some(A::from_repr(unsafe { &*ptr }))
        } else {
            None
        }
    }
}
