use std::ffi::{CStr, CString};
use std::ptr;

use crate::ffi::RED4ext;

pub type Mem = *mut autocxx::c_void;

pub trait IntoRED {
    fn into_red(self, mem: Mem);

    #[inline]
    unsafe fn set<A>(val: A, mem: Mem) {
        (mem as *mut A).write(val)
    }
}

pub trait FromRED: Sized
where
    Self::Repr: Default,
{
    type Repr;

    fn from_repr(repr: Self::Repr) -> Self;

    #[inline]
    fn from_red(frame: *mut RED4ext::CStackFrame) -> Self {
        let mut init = Self::Repr::default();
        unsafe { RED4ext::GetParameter(frame, std::mem::transmute(&mut init)) };
        Self::from_repr(init)
    }
}

macro_rules! iso_red_instances {
    ($ty:ty) => {
        impl IntoRED for $ty {
            #[inline]
            fn into_red(self, mem: Mem) {
                unsafe { Self::set(self, mem) }
            }
        }

        impl FromRED for $ty {
            type Repr = $ty;

            #[inline]
            fn from_repr(repr: Self::Repr) -> Self {
                repr
            }
        }
    };
}

iso_red_instances!(f32);
iso_red_instances!(i64);
iso_red_instances!(i32);
iso_red_instances!(i16);
iso_red_instances!(i8);
iso_red_instances!(u64);
iso_red_instances!(u32);
iso_red_instances!(u16);
iso_red_instances!(u8);

#[repr(packed)]
#[derive(Clone, Copy)]
pub struct REDString {
    data: [i8; 0x14],
    length: u32,
    _allocator: Mem,
}

impl REDString {
    fn as_str(&self) -> &str {
        unsafe {
            let ptr = if self.length < 0x40000000 {
                self.data.as_ptr()
            } else {
                *(self.data.as_ptr() as *const *const i8)
            };
            CStr::from_ptr(ptr).to_str().unwrap()
        }
    }
}

impl Default for REDString {
    fn default() -> Self {
        Self {
            data: [0; 0x14],
            length: 0,
            _allocator: ptr::null_mut(),
        }
    }
}

impl IntoRED for String {
    fn into_red(self, mem: Mem) {
        let bytes = CString::new(self).unwrap();
        let cstr = mem as *mut RED4ext::CString;
        unsafe { RED4ext::CString::ConstructAt(cstr, bytes.as_ptr(), ptr::null_mut()) };
    }
}

impl FromRED for String {
    type Repr = REDString;

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        repr.as_str().to_owned()
    }
}

#[repr(C)]
pub struct REDArray<A> {
    entries: *mut A,
    cap: u32,
    size: u32,
}

impl<A> REDArray<A> {
    fn as_slice(&self) -> &[A] {
        unsafe { std::slice::from_raw_parts(self.entries, self.size as usize) }
    }
}

impl<A> Default for REDArray<A> {
    fn default() -> Self {
        Self {
            entries: ptr::null_mut(),
            cap: 0,
            size: 0,
        }
    }
}

impl<A: FromRED + Clone> FromRED for Vec<A>
where
    A: FromRED,
    A::Repr: Clone,
{
    type Repr = REDArray<A::Repr>;

    fn from_repr(repr: Self::Repr) -> Self {
        repr.as_slice().iter().cloned().map(FromRED::from_repr).collect()
    }
}
