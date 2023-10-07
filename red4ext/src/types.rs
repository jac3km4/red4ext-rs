use std::fmt::Debug;
use std::ops::{Deref, Not};
use std::{mem, pin, ptr};

pub use ffi::IScriptable;
use red4ext_sys::ffi;
use red4ext_sys::interop::RefCount;
pub use red4ext_sys::interop::{
    CName, EntityId, GameEItemIdFlag, GamedataItemStructure, ItemId, RedString, ResRef, TweakDbId,
    Variant, VoidPtr,
};

use crate::conv::{ClassType, FromRepr, IntoRepr, NativeRepr};
use crate::rtti::Rtti;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct RedArray<A> {
    entries: *mut A,
    cap: u32,
    size: u32,
}

impl<A> RedArray<A> {
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
        let mut arr = RedArray::default();
        let ptr = VoidPtr(&mut arr as *mut _ as _);
        ffi::alloc_array(ptr, count as u32, mem::size_of::<A>() as u32);
        arr
    }

    pub fn from_sized_iter<I: ExactSizeIterator<Item = A>>(iter: I) -> Self {
        let len = iter.len();
        let mut arr: RedArray<A> = RedArray::with_capacity(len);
        for (i, elem) in iter.into_iter().enumerate() {
            unsafe { arr.entries.add(i).write(elem) }
        }
        arr.size = len as u32;
        arr
    }
}

impl<A> Default for RedArray<A> {
    #[inline]
    fn default() -> Self {
        Self {
            entries: ptr::null_mut(),
            cap: 0,
            size: 0,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Ref<A>(RefShared<A>);

impl<A> Ref<A> {
    #[inline]
    pub fn downgrade(this: &Self) -> WRef<A> {
        WRef(this.0.clone())
    }

    #[inline]
    pub fn as_shared(this: &Self) -> &RefShared<A> {
        &this.0
    }

    #[inline]
    pub fn upcast(this: Self) -> Ref<A::BaseClass>
    where
        A: ClassType,
    {
        unsafe { mem::transmute(this) }
    }
}

impl<A> Clone for Ref<A> {
    fn clone(&self) -> Self {
        unsafe { ffi::inc_ref(self.0.count) };
        Self(self.0.clone())
    }
}

impl<A> Drop for Ref<A> {
    fn drop(&mut self) {
        unsafe { &mut *self.0.count }.dec_ref();
    }
}

impl<A> Deref for Ref<A> {
    type Target = A;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.ptr }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct MaybeUninitRef<A>(RefShared<A>);

impl<A> MaybeUninitRef<A> {
    #[inline]
    pub(crate) fn new(a: RefShared<A>) -> Self {
        Self(a)
    }

    #[inline]
    pub(crate) fn get(&self) -> Option<Ref<A>> {
        self.0.ptr.is_null().not().then(|| Ref(self.0.clone()))
    }
}

impl<A> Default for MaybeUninitRef<A> {
    #[inline]
    fn default() -> Self {
        Self(RefShared::null())
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct WRef<A>(RefShared<A>);

impl<A> WRef<A> {
    #[inline]
    pub fn null() -> Self {
        Self(RefShared::null())
    }

    pub fn upgrade(self) -> Option<Ref<A>> {
        unsafe { &mut *self.0.count }
            .inc_ref_if_not_zero()
            .then(|| Ref(self.0))
    }

    #[inline]
    pub fn upcast(self) -> WRef<A::BaseClass>
    where
        A: ClassType,
    {
        unsafe { mem::transmute(self) }
    }
}

impl<A> Default for WRef<A> {
    #[inline]
    fn default() -> Self {
        Self(RefShared::null())
    }
}

impl<A> Clone for WRef<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct RefShared<A> {
    ptr: *mut A,
    count: *mut RefCount,
}

impl<A> RefShared<A> {
    #[inline]
    pub fn null() -> Self {
        Self::default()
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut A {
        self.ptr
    }

    #[inline]
    pub fn as_scriptable(&self) -> &RefShared<IScriptable> {
        unsafe { mem::transmute(self) }
    }
}

impl<A> Default for RefShared<A> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ptr::null_mut(),
            count: ptr::null_mut(),
        }
    }
}

impl<A> Clone for RefShared<A> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            count: self.count,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ScriptRef<'a, A> {
    unknown: [u8; 0x10],
    inner: *mut ffi::CBaseRttiType,
    ptr: &'a mut A,
    hash: CName,
}

impl<'a, A> ScriptRef<'a, A>
where
    A: NativeRepr,
{
    pub fn new(ptr: &'a mut A) -> Self {
        Self {
            unknown: [0; 0x10],
            inner: Rtti::get().get_type(CName::new(A::NATIVE_NAME)),
            ptr,
            hash: CName::default(),
        }
    }

    #[inline]
    pub fn as_inner(&'a self) -> &'a A {
        self.ptr
    }

    #[inline]
    pub fn as_inner_mut(&'a mut self) -> &'a mut A {
        self.ptr
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

pub trait VariantExt {
    fn new<A: IntoRepr>(val: A) -> Self;
    fn try_get<A: FromRepr>(&self) -> Option<A>;
}

impl VariantExt for Variant {
    fn new<A: IntoRepr>(val: A) -> Self {
        let mut this = Self::default();
        let typ = Rtti::get().get_type(CName::new(A::Repr::NATIVE_NAME));
        let mut repr = val.into_repr();
        unsafe {
            pin::Pin::new_unchecked(&mut this).fill(typ, VoidPtr(&mut repr as *mut _ as _));
        }
        this
    }

    fn try_get<A: FromRepr>(&self) -> Option<A> {
        if Rtti::type_name_of(self.get_type()) == CName::new(A::Repr::NATIVE_NAME) {
            let ptr = self.get_data_ptr().0 as *const <A as FromRepr>::Repr;
            Some(A::from_repr(unsafe { &*ptr }))
        } else {
            None
        }
    }
}

/// shortcut for ResRef creation.
#[macro_export]
macro_rules! res_ref {
    ($base:expr, /$lit:literal $($tt:tt)*) => {
        $crate::res_ref!($base.join($lit), $($tt)*)
    };
    ($base:expr, ) => {
        $base
    };
    ($lit:literal $($tt:tt)*) => {
        $crate::types::ResRef::new(
            &$crate::res_ref!(::std::path::Path::new($lit), $($tt)*).to_string_lossy()
        )
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn res_path() {
        use crate::res_ref;
        assert!(res_ref!("").is_err());
        assert!(res_ref!(".." / "somewhere" / "in" / "archive" / "custom.ent").is_err());
        assert!(res_ref!("base" / "somewhere" / "in" / "archive" / "custom.ent").is_ok());
        assert!(res_ref!("custom.ent").is_ok());
        assert!(res_ref!(".custom.ent").is_ok());
    }
}
