use const_combine::bounded::const_combine as combine;

use crate::types::{
    CName, EntityId, GameTime, ItemId, RedArray, RedString, Ref, ScriptClass, ScriptRef, TweakDbId,
    Variant, WeakRef,
};

/// A trait for types that can be passed across the FFI boundary to the game engine without
/// any conversion.
///
/// # Safety
///
/// Implementations of this trait are only valid if the memory representation of Self
/// is idetical to the representation of type with name Self::NAME in-game.
pub unsafe trait NativeRepr {
    const NAME: &'static str;
}

unsafe impl NativeRepr for () {
    const NAME: &'static str = "Void";
}

unsafe impl NativeRepr for RedString {
    const NAME: &'static str = "String";
}

unsafe impl<A: NativeRepr> NativeRepr for RedArray<A> {
    const NAME: &'static str = combine!("array:", A::NAME);
}

unsafe impl<A: ScriptClass> NativeRepr for Ref<A> {
    const NAME: &'static str = combine!("handle:", A::CLASS_NAME);
}

unsafe impl<A: ScriptClass> NativeRepr for WeakRef<A> {
    const NAME: &'static str = combine!("whandle:", A::CLASS_NAME);
}

unsafe impl<'a, A: NativeRepr> NativeRepr for ScriptRef<'a, A> {
    const NAME: &'static str = combine!("script_ref:", A::NAME);
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
impl_native_repr!(TweakDbId, "TweakDBID");
impl_native_repr!(ItemId, "ItemID", "gameItemID");
impl_native_repr!(EntityId, "EntityID", "entEntityID");
impl_native_repr!(GameTime, "GameTime", "GameTime");
impl_native_repr!(Variant, "Variant", "Variant");

/// A trait for types that can be converted into a representation that can be passed across
/// the FFI boundary to the game.
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
        RedString::from(self)
    }
}

impl<A> IntoRepr for Vec<A>
where
    A: IntoRepr,
{
    type Repr = RedArray<A::Repr>;

    fn into_repr(self) -> Self::Repr {
        self.into_iter().map(IntoRepr::into_repr).collect()
    }
}

/// A trait for types that can be created from a representation passed across the FFI boundary.
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

impl FromRepr for String {
    type Repr = RedString;

    #[inline]
    fn from_repr(repr: Self::Repr) -> Self {
        repr.into()
    }
}

impl<A> FromRepr for Vec<A>
where
    A: FromRepr,
{
    type Repr = RedArray<A::Repr>;

    fn from_repr(repr: Self::Repr) -> Self {
        repr.into_iter().map(FromRepr::from_repr).collect()
    }
}
