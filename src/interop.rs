use std::ffi::CStr;
use std::{mem, ptr};

use const_combine::bounded::const_combine;

use crate::{ffi, VoidPtr};

pub type Mem = *mut std::ffi::c_void;

pub trait IntoRED: Sized {
    type Repr;

    const NAME: &'static str;
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
    const NAME: &'static str;
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

    const NAME: &'static str = <Self as IsoRED>::NAME;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self
    }
}

impl IntoRED for () {
    type Repr = ();

    const NAME: &'static str = "Void";

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

impl IntoRED for String {
    type Repr = REDString;

    const NAME: &'static str = "String";

    fn into_repr(self) -> Self::Repr {
        self.as_str().into_repr()
    }
}

impl IntoRED for &str {
    type Repr = REDString;

    const NAME: &'static str = "String";

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

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [A] {
        unsafe { std::slice::from_raw_parts_mut(self.entries, self.size as usize) }
    }

    #[inline]
    pub fn with_capacity(count: usize) -> Self {
        let arr = REDArray::default();
        let ptr = unsafe { VoidPtr(mem::transmute(&arr)) };
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

impl<A: IsoRED> IsoRED for REDArray<A> {
    const NAME: &'static str = const_combine!("array:", A::NAME);
}

impl<A> FromRED for Vec<A>
where
    A: FromRED,
    A::Repr: Clone,
{
    type Repr = REDArray<A::Repr>;

    fn from_repr(repr: Self::Repr) -> Self {
        repr.as_slice().iter().cloned().map(FromRED::from_repr).collect()
    }
}

impl<A> IntoRED for Vec<A>
where
    A: IntoRED + Clone,
{
    type Repr = REDArray<A::Repr>;
    const NAME: &'static str = const_combine!("array:", A::NAME);

    fn into_repr(self) -> Self::Repr {
        REDArray::from_sized_iter(self.into_iter().map(IntoRED::into_repr))
    }
}

impl<A> IntoRED for &[A]
where
    A: IntoRED + Clone,
{
    type Repr = REDArray<A::Repr>;
    const NAME: &'static str = const_combine!("array:", A::NAME);

    fn into_repr(self) -> Self::Repr {
        REDArray::from_sized_iter(self.iter().cloned().map(IntoRED::into_repr))
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

impl<A> Copy for Ref<A> {}

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

    let mut tail = str.as_bytes();
    let mut hash = SEED;
    loop {
        match tail.split_first() {
            Some((head, rem)) => {
                hash ^= *head as u64;
                hash = hash.wrapping_mul(PRIME);
                tail = rem;
            }
            None => break hash,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Variant {
    typ: *const ffi::CBaseRTTIType,
    data: [u8; 0x10],
}

impl Variant {
    pub fn get_type(&self) -> *const ffi::CBaseRTTIType {
        self.typ
    }

    pub fn get_data(&self) -> Mem {
        ffi::Variant::get_data_ptr(self).0
    }
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
            const NAME: &'static str = $name;
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
iso_red_instance!(Ref<ffi::IScriptable>, "ref:IScriptable");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_type_names() {
        assert_eq!(<Vec<Vec<Vec<i32>>> as IntoRED>::NAME, "array:array:array:Int32");
        assert_eq!(
            <Vec<Ref<ffi::IScriptable>> as IntoRED>::NAME,
            "array:ref:IScriptable"
        );
    }

    #[test]
    fn calculate_hashes() {
        assert_eq!(CName::new("IScriptable").hash, 3191163302135919211);
        assert_eq!(CName::new("Vector2").hash, 7466804955052523504);
        assert_eq!(CName::new("Color").hash, 3769135706557701272);
    }
}
