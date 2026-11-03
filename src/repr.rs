use const_combine::bounded::const_combine as combine;

use crate::class::ScriptClass;
use crate::types::{
    CName, EntityId, GameTime, ItemId, Opt, RedArray, RedString, Ref, ScriptRef, TweakDbId,
    Variant, WeakRef,
};

/// A value whose layout matches the engine RTTI type named by `NAME`.
///
/// SDK: [RTTI types](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp).
///
/// # Safety
///
/// `Self` must match the named engine type's size, alignment, field layout,
/// validity, and ownership semantics. A matching name alone is insufficient.
pub unsafe trait NativeRepr {
    /// The engine RTTI type name, which may differ from its REDscript name.
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
    const NAME: &'static str = combine!("handle:", A::NAME);
}

unsafe impl<A: ScriptClass> NativeRepr for WeakRef<A> {
    const NAME: &'static str = combine!("whandle:", A::NAME);
}

unsafe impl<A: NativeRepr> NativeRepr for ScriptRef<'_, A> {
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
            const NAME: &'static str = $native_name;
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

/// Converts a Rust value into its engine-compatible [`NativeRepr`].
///
/// Native representations pass through unchanged; strings and vectors convert to
/// [`RedString`] and [`RedArray`], respectively.
pub trait IntoRepr: Sized {
    /// The engine-compatible representation of this value.
    type Repr: NativeRepr;

    /// Consumes this value and converts it to its engine representation.
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

impl IntoRepr for &str {
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

impl<A: NativeRepr + Default + PartialEq> IntoRepr for Opt<A> {
    type Repr = A;

    #[inline]
    fn into_repr(self) -> Self::Repr {
        match self {
            Self::Default => A::default(),
            Self::NonDefault(x) => x,
        }
    }
}

/// Converts an owned engine-compatible representation into a Rust value.
pub trait FromRepr: Sized {
    /// The engine-compatible representation consumed by this conversion.
    type Repr: NativeRepr;

    /// Consumes an engine representation and converts it to a Rust value.
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

impl<A: NativeRepr + Default + PartialEq> FromRepr for Opt<A> {
    type Repr = A;

    fn from_repr(repr: Self::Repr) -> Self {
        let repr = A::from_repr(repr);
        if repr == A::default() {
            return Self::Default;
        }
        Self::NonDefault(repr)
    }
}
