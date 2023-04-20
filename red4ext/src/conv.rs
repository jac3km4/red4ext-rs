use std::{mem, ptr};

use const_combine::bounded::const_combine;
use red4ext_sys::ffi;
use red4ext_sys::interop::Mem;

use crate::types::{CName, Color, IScriptable, REDArray, REDString, Ref, Variant, Vector2, TweakDBID};

pub trait NativeRED {
    const NAME: &'static str;
}

pub trait IntoRED: NativeRED + Sized {
    type Repr;

    fn into_repr(self) -> Self::Repr;

    #[inline]
    fn into_red(self, mem: Mem) {
        unsafe { (mem as *mut Self::Repr).write(Self::into_repr(self)) }
    }
}

pub trait FromRED: NativeRED + Sized {
    type Repr: Default;

    fn from_repr(repr: &Self::Repr) -> Self;

    #[inline]
    fn from_red(frame: *mut ffi::CStackFrame) -> Self {
        let mut init = Self::Repr::default();
        unsafe { ffi::get_parameter(frame, mem::transmute(&mut init)) };
        Self::from_repr(&init)
    }
}

pub trait IsoRED: NativeRED {}

impl<A: IsoRED + Clone + Default> FromRED for A {
    type Repr = A;

    #[inline]
    fn from_repr(repr: &Self::Repr) -> Self {
        repr.clone()
    }
}

impl<A: IsoRED> IntoRED for A {
    type Repr = A;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self
    }
}

impl NativeRED for () {
    const NAME: &'static str = "Void";
}

impl IntoRED for () {
    type Repr = ();

    #[inline]
    fn into_repr(self) -> Self::Repr {}

    #[inline]
    fn into_red(self, _mem: Mem) {}
}

impl FromRED for () {
    type Repr = ();

    #[inline]
    fn from_repr(_repr: &Self::Repr) -> Self {}

    #[inline]
    fn from_red(_frame: *mut ffi::CStackFrame) -> Self {}
}

impl NativeRED for String {
    const NAME: &'static str = "String";
}

impl IntoRED for String {
    type Repr = REDString;

    fn into_repr(self) -> Self::Repr {
        self.as_str().into_repr()
    }
}

impl NativeRED for &str {
    const NAME: &'static str = "String";
}

impl IntoRED for &str {
    type Repr = REDString;

    fn into_repr(self) -> REDString {
        let mut str = REDString::default();
        unsafe { ffi::construct_string_at(&mut str, self, ptr::null_mut()) };
        str
    }
}

impl FromRED for String {
    type Repr = REDString;

    #[inline]
    fn from_repr(repr: &Self::Repr) -> Self {
        repr.as_str().to_owned()
    }
}

impl<A: NativeRED> NativeRED for REDArray<A> {
    const NAME: &'static str = const_combine!("array:", A::NAME);
}

impl<A: IsoRED> IsoRED for REDArray<A> {}

impl<A: NativeRED> NativeRED for Vec<A> {
    const NAME: &'static str = const_combine!("array:", A::NAME);
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

impl<A> IntoRED for Vec<A>
where
    A: IntoRED,
{
    type Repr = REDArray<A::Repr>;

    fn into_repr(self) -> Self::Repr {
        REDArray::from_sized_iter(self.into_iter().map(IntoRED::into_repr))
    }
}

impl<A: NativeRED> NativeRED for &[A] {
    const NAME: &'static str = const_combine!("array:", A::NAME);
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

impl NativeRED for Variant {
    const NAME: &'static str = "Variant";
}

impl IsoRED for Variant {}

macro_rules! iso_red_instance {
    ($ty:ty, $name:literal) => {
        impl NativeRED for $ty {
            const NAME: &'static str = $name;
        }

        impl IsoRED for $ty {}
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
iso_red_instance!(TweakDBID, "TweakDBID");
iso_red_instance!(Vector2, "Vector2");
iso_red_instance!(Color, "Color");
iso_red_instance!(Ref<IScriptable>, "handle:IScriptable");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_type_names() {
        assert_eq!(
            <Vec<Vec<Vec<i32>>> as NativeRED>::NAME,
            "array:array:array:Int32"
        );
        assert_eq!(
            <Vec<Ref<IScriptable>> as NativeRED>::NAME,
            "array:handle:IScriptable"
        );
    }
}
