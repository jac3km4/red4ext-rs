use const_combine::bounded::const_combine as combine;
use red4ext_sys::ffi::{self, IScriptable};
use red4ext_sys::interop::{EntityId, ItemId, Mem};

use crate::prelude::{Ref, WRef};
use crate::types::{
    CName, Color, MaybeUninitRef, RedArray, RedString, ResRef, ScriptRef, TweakDbId, Variant,
    Vector2,
};

/// # Safety
///
/// Implementations of this trait are only valid if the memory representation of Self
/// is idetical to the representation of type with name Self::NAME in-game.
pub unsafe trait NativeRepr {
    const NAME: &'static str;
    const NATIVE_NAME: &'static str = Self::NAME;
    const MANGLED_NAME: &'static str = Self::NAME;
}

unsafe impl NativeRepr for () {
    const NAME: &'static str = "Void";
}

unsafe impl NativeRepr for RedString {
    const NAME: &'static str = "String";
}

unsafe impl<A: NativeRepr> NativeRepr for RedArray<A> {
    const MANGLED_NAME: &'static str = combine!(combine!("array<", A::MANGLED_NAME), ">");
    const NAME: &'static str = combine!("array:", A::NAME);
    const NATIVE_NAME: &'static str = combine!("array:", A::NATIVE_NAME);
}

unsafe impl NativeRepr for Variant {
    const NAME: &'static str = "Variant";
}

unsafe impl<'a, A: NativeRepr> NativeRepr for ScriptRef<'a, A> {
    const MANGLED_NAME: &'static str = combine!(combine!("script_ref<", A::MANGLED_NAME), ">");
    const NAME: &'static str = combine!("script_ref:", A::NAME);
    const NATIVE_NAME: &'static str = combine!("script_ref:", A::NATIVE_NAME);
}

unsafe impl<A: NativeRepr> NativeRepr for MaybeUninitRef<A> {
    const MANGLED_NAME: &'static str = A::MANGLED_NAME;
    const NAME: &'static str = combine!("handle:", A::NAME);
    const NATIVE_NAME: &'static str = combine!("handle:", A::NATIVE_NAME);
}

unsafe impl<A: NativeRepr> NativeRepr for WRef<A> {
    const MANGLED_NAME: &'static str = A::MANGLED_NAME;
    const NAME: &'static str = combine!("whandle:", A::NAME);
    const NATIVE_NAME: &'static str = combine!("whandle:", A::NATIVE_NAME);
}

pub trait IntoRepr: Sized {
    type Repr: NativeRepr;

    fn into_repr(self) -> Self::Repr;
}

impl<A: NativeRepr> IntoRepr for A {
    type Repr = A;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self
    }
}

impl IntoRepr for String {
    type Repr = RedString;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        self.as_str().into_repr()
    }
}

impl IntoRepr for &str {
    type Repr = RedString;

    #[inline]
    fn into_repr(self) -> RedString {
        RedString::new(self)
    }
}

impl<A> IntoRepr for Vec<A>
where
    A: IntoRepr,
{
    type Repr = RedArray<A::Repr>;

    fn into_repr(self) -> Self::Repr {
        RedArray::from_sized_iter(self.into_iter().map(IntoRepr::into_repr))
    }
}

impl<A> IntoRepr for &[A]
where
    A: IntoRepr + Clone,
{
    type Repr = RedArray<A::Repr>;

    fn into_repr(self) -> Self::Repr {
        RedArray::from_sized_iter(self.iter().cloned().map(IntoRepr::into_repr))
    }
}

impl<A: NativeRepr> IntoRepr for Ref<A> {
    type Repr = MaybeUninitRef<A>;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        MaybeUninitRef::new(Ref::as_shared(&self).clone())
    }
}

pub trait FromRepr: Sized {
    type Repr: NativeRepr;

    fn from_repr(repr: &Self::Repr) -> Self;
}

impl<A: NativeRepr + Clone> FromRepr for A {
    type Repr = A;

    #[inline]
    fn from_repr(repr: &Self::Repr) -> Self {
        repr.clone()
    }
}

impl FromRepr for String {
    type Repr = RedString;

    #[inline]
    fn from_repr(repr: &Self::Repr) -> Self {
        repr.as_str().to_owned()
    }
}

impl<A> FromRepr for Vec<A>
where
    A: FromRepr,
{
    type Repr = RedArray<A::Repr>;

    fn from_repr(repr: &Self::Repr) -> Self {
        repr.as_slice().iter().map(FromRepr::from_repr).collect()
    }
}

impl<A: NativeRepr> FromRepr for Ref<A> {
    type Repr = MaybeUninitRef<A>;

    fn from_repr(repr: &Self::Repr) -> Self {
        repr.get().expect("ref was uninitialized")
    }
}

macro_rules! impl_native_repr {
    ($ty:ty, $name:literal) => {
        unsafe impl NativeRepr for $ty {
            const NAME: &'static str = $name;
        }
    };
    ($ty:ty, $name:literal, $native_name:literal) => {
        unsafe impl NativeRepr for $ty {
            const NAME: &'static str = $name;
            const NATIVE_NAME: &'static str = $native_name;
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
impl_native_repr!(ResRef, "ResRef", "redResourceReferenceScriptToken");
impl_native_repr!(TweakDbId, "TweakDBID");
impl_native_repr!(ItemId, "ItemID", "gameItemID");
impl_native_repr!(EntityId, "EntityID", "entEntityID");
impl_native_repr!(Vector2, "Vector2");
impl_native_repr!(Color, "Color");
impl_native_repr!(IScriptable, "IScriptable");

#[inline]
pub(crate) fn fill_memory<A: IntoRepr>(val: A, mem: Mem) {
    unsafe { (mem as *mut A::Repr).write(val.into_repr()) }
}

#[inline]
pub(crate) fn from_frame<A>(frame: *mut ffi::CStackFrame) -> A
where
    A: FromRepr,
    A::Repr: Default,
{
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
            <Vec<Vec<Vec<i32>>> as IntoRepr>::Repr::NAME,
            "array:array:array:Int32"
        );
        assert_eq!(
            <Vec<Ref<IScriptable>> as IntoRepr>::Repr::NAME,
            "array:handle:IScriptable"
        );
        assert_eq!(
            <Vec<Ref<IScriptable>> as IntoRepr>::Repr::MANGLED_NAME,
            "array<IScriptable>"
        );
        assert_eq!(
            <Vec<ItemId> as IntoRepr>::Repr::NATIVE_NAME,
            "array:gameItemID"
        );
    }
}
