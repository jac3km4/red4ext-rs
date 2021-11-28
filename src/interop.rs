use std::ffi::{CStr, CString};
use std::{mem, ptr};

use cxx::UniquePtr;

use crate::ffi::RED4ext;

pub type Mem = *mut autocxx::c_void;

pub trait IntoRED: Sized {
    type Repr;

    fn type_name() -> &'static str;
    fn into_repr(self) -> Self::Repr;

    #[inline]
    fn into_red(self, mem: Mem) {
        unsafe { (mem as *mut Self::Repr).write(Self::into_repr(self)) }
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
        unsafe { RED4ext::GetParameter(frame, mem::transmute(&mut init)) };
        Self::from_repr(init)
    }
}

impl IntoRED for () {
    type Repr = ();

    #[inline]
    fn type_name() -> &'static str {
        "Void"
    }

    #[inline]
    fn into_repr(self) -> Self::Repr {}
    #[inline]
    fn into_red(self, _mem: Mem) {}
}

impl FromRED for () {
    type Repr = ();

    #[inline]
    fn from_repr(_repr: Self::Repr) -> Self {}
    #[inline]
    fn from_red(_frame: *mut RED4ext::CStackFrame) -> Self {}
}

macro_rules! iso_red_instances {
    ($ty:ty, $name:ident) => {
        impl IntoRED for $ty {
            type Repr = $ty;

            #[inline]
            fn type_name() -> &'static str {
                stringify!($name)
            }

            #[inline]
            fn into_repr(self) -> Self::Repr {
                self
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

iso_red_instances!(f32, Float);
iso_red_instances!(f64, Double);
iso_red_instances!(i64, Int64);
iso_red_instances!(i32, Int32);
iso_red_instances!(i16, Int16);
iso_red_instances!(i8, Int8);
iso_red_instances!(u64, Uint64);
iso_red_instances!(u32, Uint32);
iso_red_instances!(u16, Uint16);
iso_red_instances!(u8, Uint8);
iso_red_instances!(bool, Bool);

#[repr(C, packed)]
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
    #[inline]
    fn default() -> Self {
        Self {
            data: [0; 0x14],
            length: 0,
            _allocator: ptr::null_mut(),
        }
    }
}

impl IntoRED for String {
    type Repr = REDString;

    #[inline]
    fn type_name() -> &'static str {
        "String"
    }

    fn into_repr(self) -> REDString {
        let bytes = CString::new(self).unwrap();
        let mut str = REDString::default();
        unsafe { RED4ext::CString::ConstructAt(mem::transmute(&mut str), bytes.as_ptr(), ptr::null_mut()) };
        str
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
    #[inline]
    fn as_slice(&self) -> &[A] {
        unsafe { std::slice::from_raw_parts(self.entries, self.size as usize) }
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

#[repr(C)]
#[derive(Default)]
pub struct RefCount {
    strong_refs: u32,
    weak_refs: u32,
}

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

impl FromRED for Ref<RED4ext::IScriptable> {
    type Repr = Self;

    fn from_repr(repr: Self::Repr) -> Self {
        repr
    }
}

impl IntoRED for Ref<RED4ext::IScriptable> {
    type Repr = Self;

    fn type_name() -> &'static str {
        "ref<IScriptable>"
    }

    fn into_repr(self) -> Self::Repr {
        self
    }
}

pub type CName = UniquePtr<RED4ext::CName>;

#[inline]
pub const fn fnv1a64(str: &str) -> u64 {
    const PRIME: u64 = 0x100000001b3;
    const SEED: u64 = 0xCBF29CE484222325;

    #[inline]
    const fn calc(str: &[u8], mut hash: u64) -> u64 {
        match str.split_first() {
            Some((head, tail)) => {
                hash ^= *head as u64;
                hash *= PRIME;
                calc(tail, hash)
            }
            None => hash,
        }
    }

    calc(str.as_bytes(), SEED)
}

#[repr(C)]
pub struct StackArg {
    typ: *const RED4ext::CBaseRTTIType,
    value: Mem,
}

impl StackArg {
    #[inline]
    pub fn new(typ: *const RED4ext::CBaseRTTIType, value: Mem) -> StackArg {
        StackArg { typ, value }
    }
}
