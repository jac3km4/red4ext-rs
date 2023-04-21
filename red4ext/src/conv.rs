use const_combine::bounded::const_combine;
use red4ext_sys::ffi;
use red4ext_sys::interop::Mem;

use crate::types::{
    CName, Color, IScriptable, REDArray, REDString, Ref, TweakDBID, Variant, Vector2,
};

pub unsafe trait NativeRepr: Default {
    const NAME: &'static str;
}

unsafe impl NativeRepr for () {
    const NAME: &'static str = "Void";
}

unsafe impl NativeRepr for REDString {
    const NAME: &'static str = "String";
}

unsafe impl<A: NativeRepr> NativeRepr for REDArray<A> {
    const NAME: &'static str = const_combine!("array:", A::NAME);
}

unsafe impl NativeRepr for Variant {
    const NAME: &'static str = "Variant";
}

pub trait IntoRED: Sized {
    type Repr: NativeRepr;

    fn into_repr(self) -> Self::Repr;
}

impl<A: NativeRepr> IntoRED for A {
    type Repr = A;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self
    }
}

impl IntoRED for String {
    type Repr = REDString;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self.as_str().into_repr()
    }
}

impl IntoRED for &str {
    type Repr = REDString;

    #[inline]
    fn into_repr(self) -> REDString {
        REDString::new(self)
    }
}

impl<A> IntoRED for Vec<A>
where
    A: IntoRED,
{
    type Repr = REDArray<A::Repr>;

    fn into_repr(self) -> Self::Repr {
        REDArray::from_sized_iter(self.into_iter().map(IntoRED::into_repr))
    }
}

impl<A> IntoRED for &[A]
where
    A: IntoRED + Clone,
{
    type Repr = REDArray<A::Repr>;

    fn into_repr(self) -> Self::Repr {
        REDArray::from_sized_iter(self.iter().cloned().map(IntoRED::into_repr))
    }
}

pub trait FromRED: Sized {
    type Repr: NativeRepr;

    fn from_repr(repr: &Self::Repr) -> Self;
}

impl<A: NativeRepr + Clone> FromRED for A {
    type Repr = A;

    #[inline]
    fn from_repr(repr: &Self::Repr) -> Self {
        repr.clone()
    }
}

impl FromRED for String {
    type Repr = REDString;

    #[inline]
    fn from_repr(repr: &Self::Repr) -> Self {
        repr.as_str().to_owned()
    }
}

impl<A> FromRED for Vec<A>
where
    A: FromRED,
{
    type Repr = REDArray<A::Repr>;

    fn from_repr(repr: &Self::Repr) -> Self {
        repr.as_slice().iter().map(FromRED::from_repr).collect()
    }
}

macro_rules! impl_native_repr {
    ($ty:ty, $name:literal) => {
        unsafe impl NativeRepr for $ty {
            const NAME: &'static str = $name;
        }
    };
}

impl_native_repr!(f32, "Float");
impl_native_repr!(f64, "Double");
impl_native_repr!(i64, "Int64");
impl_native_repr!(i32, "Int32");
impl_native_repr!(i16, "Int16");
impl_native_repr!(i8, "Int8");
impl_native_repr!(u64, "Uint64");
impl_native_repr!(u32, "Uint32");
impl_native_repr!(u16, "Uint16");
impl_native_repr!(u8, "Uint8");
impl_native_repr!(bool, "Bool");
impl_native_repr!(CName, "CName");
impl_native_repr!(TweakDBID, "TweakDBID");
impl_native_repr!(Vector2, "Vector2");
impl_native_repr!(Color, "Color");
impl_native_repr!(Ref<IScriptable>, "handle:IScriptable");

#[inline]
pub(crate) fn fill_memory<A: IntoRED>(val: A, mem: Mem) {
    unsafe { (mem as *mut A::Repr).write(val.into_repr()) }
}

#[inline]
pub(crate) fn from_frame<A: FromRED>(frame: *mut ffi::CStackFrame) -> A {
    let mut init = A::Repr::default();
    unsafe { ffi::get_parameter(frame, std::mem::transmute(&mut init)) };
    A::from_repr(&init)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_type_names() {
        assert_eq!(
            <Vec<Vec<Vec<i32>>> as IntoRED>::Repr::NAME,
            "array:array:array:Int32"
        );
        assert_eq!(
            <Vec<Ref<IScriptable>> as IntoRED>::Repr::NAME,
            "array:handle:IScriptable"
        );
    }
}
