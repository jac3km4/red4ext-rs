use const_combine::bounded::const_combine as combine;

use crate::types::{
    CName, EntityId, ItemId, RedArray, RedString, Ref, ResRef, ScriptClass, ScriptRef, TweakDbId,
    WeakRef,
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

unsafe impl<A: ScriptClass> NativeRepr for Ref<A> {
    const MANGLED_NAME: &'static str = A::CLASS_NAME;
    const NAME: &'static str = combine!("handle:", A::CLASS_NAME);
    const NATIVE_NAME: &'static str = combine!("handle:", A::NATIVE_NAME);
}

unsafe impl<A: ScriptClass> NativeRepr for WeakRef<A> {
    const MANGLED_NAME: &'static str = A::CLASS_NAME;
    const NAME: &'static str = combine!("whandle:", A::CLASS_NAME);
    const NATIVE_NAME: &'static str = combine!("whandle:", A::NATIVE_NAME);
}

unsafe impl<'a, A: NativeRepr> NativeRepr for ScriptRef<'a, A> {
    const MANGLED_NAME: &'static str = combine!(combine!("script_ref<", A::MANGLED_NAME), ">");
    const NAME: &'static str = combine!("script_ref:", A::NAME);
    const NATIVE_NAME: &'static str = combine!("script_ref:", A::NATIVE_NAME);
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

pub trait FromRepr: Sized {
    type Repr: NativeRepr;

    fn from_repr(repr: Self::Repr) -> Self;
}

impl<A: NativeRepr> FromRepr for A {
    type Repr = A;

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        repr
    }
}
