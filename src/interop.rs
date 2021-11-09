use std::ffi::{CStr, CString};
use std::pin::Pin;
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

pub trait FromRED {
    fn from_red(frame: *mut RED4ext::CStackFrame) -> Self;

    #[inline]
    unsafe fn get<I: Default>(frame: *mut RED4ext::CStackFrame) -> I {
        let mut init = I::default();
        RED4ext::GetParameter(frame, std::mem::transmute(&mut init));
        init
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
            #[inline]
            fn from_red(frame: *mut RED4ext::CStackFrame) -> Self {
                unsafe { Self::get(frame) }
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

impl IntoRED for String {
    fn into_red(self, mem: Mem) {
        let bytes = CString::new(self).unwrap();
        let cstr = mem as *mut RED4ext::CString;
        unsafe { RED4ext::CString::ConstructAt(cstr, bytes.as_ptr(), ptr::null_mut()) };
    }
}

impl FromRED for String {
    fn from_red(frame: *mut RED4ext::CStackFrame) -> Self {
        unsafe {
            let cstr = RED4ext::CString::make_unique(ptr::null_mut()).into_raw();
            RED4ext::GetParameter(frame, std::mem::transmute(cstr));

            CStr::from_ptr(Pin::new_unchecked(cstr.as_mut().unwrap()).c_str())
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[repr(C)]
struct REDArray<A> {
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

impl<A: FromRED + Clone> FromRED for Vec<A> {
    fn from_red(frame: *mut RED4ext::CStackFrame) -> Self {
        unsafe {
            let mut arr = REDArray::default();
            RED4ext::GetParameter(frame, std::mem::transmute(&mut arr));
            arr.as_slice().to_vec()
        }
    }
}
