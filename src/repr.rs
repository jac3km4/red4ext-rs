use const_combine::bounded::const_combine as combine;

use crate::types::{RedArray, RedString, Ref, ScriptClass, ScriptRef, WeakRef};

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
