use std::ffi::CStr;
use std::{mem, ptr};

use crate::ffi;

pub type Mem = *mut std::ffi::c_void;

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
    fn from_red(frame: *mut ffi::CStackFrame) -> Self {
        let mut init = Self::Repr::default();
        unsafe { ffi::get_parameter(frame, mem::transmute(&mut init)) };
        Self::from_repr(init)
    }
}

pub trait IsoRED: Default {
    fn type_name() -> &'static str;
}

impl<A: IsoRED> FromRED for A {
    type Repr = A;

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        repr
    }
}

impl<A: IsoRED> IntoRED for A {
    type Repr = A;

    #[inline]
    fn type_name() -> &'static str {
        <Self as IsoRED>::type_name()
    }

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self
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
    fn from_red(_frame: *mut ffi::CStackFrame) -> Self {}
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
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

impl IntoRED for &str {
    type Repr = REDString;

    #[inline]
    fn type_name() -> &'static str {
        "String"
    }

    fn into_repr(self) -> Self::Repr {
        self.to_owned().into_repr()
    }
}

impl IntoRED for String {
    type Repr = REDString;

    #[inline]
    fn type_name() -> &'static str {
        "String"
    }

    fn into_repr(self) -> REDString {
        let mut str = REDString::default();
        unsafe { ffi::construct_string_at(&mut str, &self, ptr::null_mut()) };
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

#[derive(Debug)]
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

#[derive(Debug, Default)]
#[repr(C)]
pub struct RefCount {
    strong_refs: u32,
    weak_refs: u32,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CName {
    hash: u64,
}

impl CName {
    #[inline]
    pub const fn new(str: &str) -> Self {
        Self { hash: fnv1a64(str) }
    }
}

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

#[derive(Debug)]
#[repr(C)]
pub struct StackArg {
    typ: *const ffi::CBaseRTTIType,
    value: Mem,
}

impl StackArg {
    #[inline]
    pub fn new(typ: *const ffi::CBaseRTTIType, value: Mem) -> StackArg {
        StackArg { typ, value }
    }
}

macro_rules! iso_red_instance {
    ($ty:ty, $name:literal) => {
        impl IsoRED for $ty {
            #[inline]
            fn type_name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

iso_red_instance!(f32, "Float");
iso_red_instance!(f64, "Double");
iso_red_instance!(i64, "Int64");
iso_red_instance!(i32, "Int32");
iso_red_instance!(i16, "Int16");
iso_red_instance!(i8, "Int8");
iso_red_instance!(u64, "Uint64");
iso_red_instance!(u32, "Uint32");
iso_red_instance!(u16, "Uint16");
iso_red_instance!(u8, "Uint8");
iso_red_instance!(bool, "Bool");
iso_red_instance!(CName, "CName");
iso_red_instance!(Vector2, "Vector2");
iso_red_instance!(Color, "Color");
iso_red_instance!(Ref<ffi::IScriptable>, "ref<IScriptable>");
