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

/// A dynamic array container. Corresponds to `array` in redscript.
#[derive(Debug)]
#[repr(C)]
pub struct RedArray<A> {
    entries: *mut A,
    cap: u32,
    size: u32,
}

impl<A> RedArray<A> {
    /// Allocates a new array with the given capacity.
    #[inline]
    pub fn with_capacity(count: usize) -> Self {
        let mut arr = RedArray::default();
        let ptr = VoidPtr(&mut arr as *mut _ as _);
        ffi::alloc_array(ptr, count as u32, mem::size_of::<A>() as u32);
        arr
    }

    /// Creates a new array from an iterator.
    pub fn from_sized_iter<I: ExactSizeIterator<Item = A>>(iter: I) -> Self {
        let len = iter.len();
        let mut arr: RedArray<A> = RedArray::with_capacity(len);
        for (i, elem) in iter.into_iter().enumerate() {
            unsafe { arr.entries.add(i).write(elem) }
        }
        arr.size = len as u32;
        arr
    }

    /// Retrieves the contents of this array as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[A] {
        let ptr = self
            .entries
            .is_null()
            .then(|| ptr::NonNull::dangling().as_ptr())
            .unwrap_or(self.entries);
        unsafe { std::slice::from_raw_parts(ptr, self.size as usize) }
    }

    /// Retrieves the contents of this array as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [A] {
        let ptr = self
            .entries
            .is_null()
            .then(|| ptr::NonNull::dangling().as_ptr())
            .unwrap_or(self.entries);
        unsafe { std::slice::from_raw_parts_mut(ptr, self.size as usize) }
    }

    /// Returns an iterator over the elements of this array.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &A> {
        self.as_slice().iter()
    }
}

impl<A> Drop for RedArray<A> {
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(self.as_mut_slice()) };
        ffi::free_array(VoidPtr(self.entries as _), mem::size_of::<A>());
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

impl<A> AsRef<[A]> for RedArray<A> {
    #[inline]
    fn as_ref(&self) -> &[A] {
        self.as_slice()
    }
}

impl<A: Clone> From<&[A]> for RedArray<A> {
    fn from(value: &[A]) -> Self {
        Self::from_sized_iter(value.iter().cloned())
    }
}

impl<A> IntoIterator for RedArray<A> {
    type IntoIter = IntoIter<A>;
    type Item = A;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            array: mem::ManuallyDrop::new(self),
            index: 0,
        }
    }
}

impl<'a, A> IntoIterator for &'a RedArray<A> {
    type IntoIter = std::slice::Iter<'a, A>;
    type Item = &'a A;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

#[derive(Debug)]
pub struct IntoIter<A> {
    array: mem::ManuallyDrop<RedArray<A>>,
    index: usize,
}

impl<A> Iterator for IntoIter<A> {
    type Item = A;

    fn next(&mut self) -> Option<Self::Item> {
        (self.index < self.array.size as usize).then(|| unsafe {
            let current = self.index;
            self.index += 1;
            self.array.entries.add(current).read()
        })
    }
}

impl<A> Drop for IntoIter<A> {
    fn drop(&mut self) {
        if self.array.entries.is_null() {
            return;
        }
        unsafe {
            // drop the remaining elements starting at self.index
            let rest = self.array.entries.add(self.index);
            let count = self.array.size as usize - self.index;
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(rest, count));
        };
        // drop the underlying buffer
        ffi::free_array(VoidPtr(&mut self.array as *mut _ as _), mem::size_of::<A>());
    }
}

/// A strong reference to a scripted class instance. Corresponds to `ref` in redscript.
#[derive(Debug)]
#[repr(C)]
pub struct Ref<A>(RefShared<A>);

impl<A> Ref<A> {
    /// Creates a new weak reference pointing at the same instance as this [`Ref`].
    #[inline]
    pub fn downgrade(this: &Self) -> WRef<A> {
        WRef(this.0.clone())
    }

    #[doc(hidden)]
    #[inline]
    pub fn as_shared(this: &Self) -> &RefShared<A> {
        &this.0
    }

    /// Casts this reference to a reference to it's base class.
    #[inline]
    pub fn upcast(this: Self) -> Ref<A::BaseClass>
    where
        A: ClassType,
    {
        unsafe { mem::transmute(this) }
    }
}

impl<A> Drop for Ref<A> {
    fn drop(&mut self) {
        if unsafe { &mut *self.0.count }.dec_ref() {
            let instance = self.0.as_scriptable().as_ptr();
            let allocator = unsafe { pin::Pin::new_unchecked(&mut *instance) }.get_allocator();
            unsafe { pin::Pin::new_unchecked(&mut *allocator) }.free(VoidPtr(instance as _));
        }
    }
}

impl<A> Clone for Ref<A> {
    fn clone(&self) -> Self {
        unsafe { ffi::inc_ref(self.0.count) };
        Self(self.0.clone())
    }
}

impl<A> Deref for Ref<A> {
    type Target = A;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.ptr }
    }
}

/// A weak reference to a scripted class instance. Corresponds to `wref` in redscript.
#[derive(Debug)]
#[repr(C)]
pub struct WRef<A>(RefShared<A>);

impl<A> WRef<A> {
    /// Creates a null weak reference.
    #[inline]
    pub fn null() -> Self {
        Self(RefShared::null())
    }

    /// Attempts to upgrade this weak reference to a strong reference.
    /// Returns [`Ref`] if the instance is still alive, or `None` if it has been dropped.
    pub fn upgrade(self) -> Option<Ref<A>> {
        unsafe { &mut *self.0.count }
            .inc_ref_if_not_zero()
            .then(|| Ref(self.0))
    }

    /// Casts this reference to a reference to it's base class.
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
pub struct MaybeUninitRef<A>(RefShared<A>);

impl<A> MaybeUninitRef<A> {
    #[inline]
    pub(crate) fn new(a: RefShared<A>) -> Self {
        Self(a)
    }

    #[inline]
    pub fn into_ref(self) -> Option<Ref<A>> {
        self.0.ptr.is_null().not().then(|| Ref(self.0))
    }
}

impl<A> Default for MaybeUninitRef<A> {
    #[inline]
    fn default() -> Self {
        Self(RefShared::null())
    }
}

#[doc(hidden)]
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

/// A reference to data originating from a script. Corresponds to `script_ref` in redscript.
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
    /// Creates a new [`Variant`] from a value.
    fn new<A: IntoRepr>(val: A) -> Self;

    /// Takes the value out of this [`Variant`] if the type `A` matches what's inside. On success, the
    /// [`Variant`] is emptied. On failure, the [`Variant`] is left unchanged.
    fn try_take<A: FromRepr>(&mut self) -> Option<A>;
}

impl VariantExt for Variant {
    fn new<A: IntoRepr>(val: A) -> Self {
        let mut this = Self::default();
        let typ = Rtti::get().get_type(CName::new(A::Repr::NATIVE_NAME));
        // Variant owns the repr, so we need to prevent the compiler from dropping it.
        let mut repr = mem::ManuallyDrop::new(val.into_repr());
        unsafe {
            pin::Pin::new_unchecked(&mut this).fill(typ, VoidPtr(&mut repr as *mut _ as _));
        }
        this
    }

    fn try_take<A: FromRepr>(&mut self) -> Option<A> {
        if Rtti::type_name_of(self.get_type()) == Some(CName::new(A::Repr::NATIVE_NAME)) {
            let ptr = self.get_data_ptr().0 as *const <A as FromRepr>::Repr;
            let value = unsafe { ptr.read() };
            // We use ptr::write to prevent the compiler from dropping the Variant.
            // Otherwise we would have a double free of the repr.
            unsafe { ptr::write(self, Self::undefined()) }
            Some(A::from_repr(value))
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
