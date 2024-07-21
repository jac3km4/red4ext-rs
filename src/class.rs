use sealed::sealed;

use crate::types::{IScriptable, Ref};

/// A trait for types that represent script classes.
///
/// # Safety
/// Implementors must ensure that the type's layout is compatible with the layout of the native
/// type that has the specified name.
pub unsafe trait ScriptClass: Sized {
    type Kind: ClassKind<Self>;

    const NAME: &'static str;
}

/// A trait for distinguishing between native and scripted classes.
#[sealed]
pub trait ClassKind<T> {
    type NativeType;

    fn fields(inst: &Self::NativeType) -> &T;
    fn fields_mut(inst: &mut Self::NativeType) -> &mut T;
}

/// Marker types for distinguishing between native and scripted classes.
pub mod class_kind {
    use super::*;

    /// A marker type for scripted classes. Scripted classes are stored with an additional level of
    /// indirection inside of [`IScriptable`].
    #[derive(Debug)]
    pub struct Scripted;

    #[sealed]
    impl<T> ClassKind<T> for Scripted {
        type NativeType = IScriptable;

        #[inline]
        fn fields(inst: &Self::NativeType) -> &T {
            unsafe { &*inst.fields().as_ptr().cast::<T>() }
        }

        #[inline]
        fn fields_mut(inst: &mut Self::NativeType) -> &mut T {
            unsafe { &mut *inst.fields().as_ptr().cast::<T>() }
        }
    }

    /// A marker type for native classes. Native classes are represented directly by their native type.
    #[derive(Debug)]
    pub struct Native;

    #[sealed]
    impl<T> ClassKind<T> for Native {
        type NativeType = T;

        #[inline]
        fn fields(inst: &Self::NativeType) -> &T {
            inst
        }

        #[inline]
        fn fields_mut(inst: &mut Self::NativeType) -> &mut T {
            inst
        }
    }
}

/// A trait for operations on script classes.
#[sealed]
pub trait ScriptClassOps: ScriptClass {
    /// Creates a new reference to the class.
    fn new_ref() -> Option<Ref<Self>>;
    /// Creates a new reference to the class and initializes it with the provided function.
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

pub type NativeType<T> = <<T as ScriptClass>::Kind as ClassKind<T>>::NativeType;
