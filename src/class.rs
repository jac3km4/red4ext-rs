use sealed::sealed;

use crate::types::{IScriptable, Ref};

/// Maps Rust fields to a native or scripted RTTI class.
///
/// SDK: [IScriptable](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/IScriptable.hpp).
///
/// # Safety
/// `NAME` and `Kind` must identify the actual engine class. Native types must
/// include their base layout; scripted types must match the separate script-field
/// storage. Size, alignment, field validity, and cleanup must match the engine.
pub unsafe trait ScriptClass: Sized {
    /// Whether `Self` describes the native object or its separate script fields.
    type Kind: ClassKind<Self>;

    /// The engine RTTI class name, which may differ from its REDscript name.
    const NAME: &'static str;
}

/// Selects how a class instance is borrowed as Rust fields.
#[sealed]
pub trait ClassKind<T> {
    /// The object type held by [`Ref`], before projecting its fields.
    type NativeType;

    /// Projects the native instance into its Rust field layout.
    ///
    /// # Safety
    /// The instance must belong to the class bound to `T`, with initialized,
    /// correctly aligned storage for `T` for the duration of the borrow.
    unsafe fn fields(inst: &Self::NativeType) -> &T;
    /// Projects the native instance into its mutable Rust field layout.
    ///
    /// # Safety
    /// The requirements of `fields` apply, and the field storage must be
    /// exclusively accessible for the duration of the borrow.
    unsafe fn fields_mut(inst: &mut Self::NativeType) -> &mut T;
}

/// Marker types for distinguishing between native and scripted classes.
pub mod class_kind {
    use super::*;

    /// A class whose Rust layout describes script fields stored separately from [`IScriptable`].
    #[derive(Debug)]
    pub struct Scripted;

    #[sealed]
    impl<T> ClassKind<T> for Scripted {
        type NativeType = IScriptable;

        #[inline]
        unsafe fn fields(inst: &Self::NativeType) -> &T {
            unsafe { &*inst.fields().as_ptr().cast::<T>() }
        }

        #[inline]
        unsafe fn fields_mut(inst: &mut Self::NativeType) -> &mut T {
            unsafe { &mut *inst.fields().as_ptr().cast::<T>() }
        }
    }

    /// A class whose Rust layout includes the native object and its base class.
    #[derive(Debug)]
    pub struct Native;

    #[sealed]
    impl<T> ClassKind<T> for Native {
        type NativeType = T;

        #[inline]
        unsafe fn fields(inst: &Self::NativeType) -> &T {
            inst
        }

        #[inline]
        unsafe fn fields_mut(inst: &mut Self::NativeType) -> &mut T {
            inst
        }
    }
}

/// Instance construction for types implementing [`ScriptClass`].
#[sealed]
pub trait ScriptClassOps: ScriptClass {
    /// Constructs an engine instance and returns a strong reference to it.
    /// Returns `None` if the class is not registered or no instance is available.
    /// Requires initialized engine RTTI.
    fn new_ref() -> Option<Ref<Self>>;
    /// Constructs an engine instance, then initializes its fields with `init`.
    /// Returns `None` if the class is not registered or no instance is available.
    /// Requires initialized engine RTTI.
    fn new_ref_with(init: impl FnOnce(&mut Self)) -> Option<Ref<Self>>;
}

#[sealed]
impl<T: ScriptClass> ScriptClassOps for T {
    #[inline]
    fn new_ref() -> Option<Ref<Self>> {
        Ref::new()
    }

    #[inline]
    fn new_ref_with(init: impl FnOnce(&mut Self)) -> Option<Ref<Self>> {
        Ref::new_with(init)
    }
}

/// The object type held by a reference to `T`, before projecting its fields.
pub type NativeType<T> = <<T as ScriptClass>::Kind as ClassKind<T>>::NativeType;
