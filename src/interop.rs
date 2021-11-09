use std::ffi::{CStr, CString};
use std::pin::Pin;
use std::ptr;

use crate::ffi::RED4ext;

pub trait IntoRED {
    fn into_red(self, mem: *mut autocxx::c_void);

    unsafe fn set<A>(val: A, meme: *mut autocxx::c_void) {
        (meme as *mut A).write(val)
    }
}

pub trait FromRED {
    fn from_red(frame: *mut RED4ext::CStackFrame) -> Self;

    unsafe fn get<I: Default>(frame: *mut RED4ext::CStackFrame) -> I {
        let mut init = I::default();
        RED4ext::GetParameter(frame, std::mem::transmute(&mut init));
        init
    }
}

macro_rules! iso_red_instances {
    ($ty:ty) => {
        impl IntoRED for $ty {
            fn into_red(self, mem: *mut autocxx::c_void) {
                unsafe { Self::set(self, mem) }
            }
        }

        impl FromRED for $ty {
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
    fn into_red(self, mem: *mut autocxx::c_void) {
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
