use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{mem, ptr};

use super::{CName, ISerializable, Type};
use crate::class::{NativeType, ScriptClass};
use crate::raw::root::RED4ext as red;
use crate::repr::NativeRepr;
use crate::systems::RttiSystem;
use crate::types::PtrEq;
use crate::{ClassKind, VoidPtr};

/// A strong, reference-counted handle to a class instance (`ref<T>` in REDscript).
///
/// Cloning shares the same instance rather than copying its fields. A default
/// handle is null; a non-null handle keeps its instance alive until the last
/// strong handle is dropped. Use [`WeakRef`] to refer to an instance without
/// keeping it alive.
///
/// Unlike [`ScriptRef`], this owns a share of the instance. Field access is unsafe
/// because the engine and other handles may access the same data.
///
/// SDK: [Handle](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Handle.hpp).
#[repr(transparent)]
pub struct Ref<T: ScriptClass>(BaseRef<NativeType<T>>);

impl<T: ScriptClass> Ref<T> {
    /// Allocates and constructs an instance of `T` through engine RTTI.
    ///
    /// Returns `None` if the class is not registered or no instance is available.
    /// Requires initialized engine RTTI and allocators.
    #[inline]
    pub fn new() -> Option<Self> {
        Self::new_with(|_| {})
    }

    /// Constructs an instance as in [`Self::new`], then initializes its fields.
    /// The callback runs before the new handle is returned.
    pub fn new_with(init: impl FnOnce(&mut T)) -> Option<Self> {
        let system = RttiSystem::get();
        let class = system.get_class(CName::new(T::NAME))?;
        let mut this = Self::default();
        Self::ctor(&mut this, class.instantiate().as_ptr().cast::<T>());

        // The unsafe ScriptClass implementation supplies the field layout; this
        // newly constructed instance has not escaped to another Rust handle.
        init(unsafe { T::Kind::fields_mut(this.0.instance_mut()?) });
        Some(this)
    }

    fn ctor(this: *mut Self, data: *mut T) {
        unsafe {
            let ctor = crate::fn_from_hash!(Handle_ctor, unsafe extern "C" fn(VoidPtr, VoidPtr));
            ctor(this as *mut _ as VoidPtr, data as *mut _ as VoidPtr);
        }
    }

    /// Borrows `T`'s fields, or returns `None` for a null handle.
    /// For scripted classes these are separate from the native instance.
    ///
    /// # Safety
    /// Prevent mutation through the engine or other handles for the entire borrow.
    #[inline]
    pub unsafe fn fields(&self) -> Option<&T> {
        Some(unsafe { T::Kind::fields(self.0.instance()?) })
    }

    /// Mutably borrows `T`'s fields, or returns `None` for a null handle.
    ///
    /// # Safety
    /// Ensure exclusive access: no engine or other handle may read or write these
    /// fields for the entire borrow.
    #[inline]
    pub unsafe fn fields_mut(&mut self) -> Option<&mut T> {
        Some(unsafe { T::Kind::fields_mut(self.0.instance_mut()?) })
    }

    /// Borrows the native instance, or returns `None` for a null handle.
    /// For scripted classes this is the native base, not the scripted fields.
    ///
    /// # Safety
    /// Prevent mutation through the engine or other handles for the entire borrow.
    #[inline]
    pub unsafe fn instance(&self) -> Option<&NativeType<T>> {
        self.0.instance()
    }

    /// Consumes this strong handle and returns a weak handle to the same instance.
    ///
    /// If this was the last strong handle, the instance is destroyed and the
    /// returned weak handle cannot be upgraded. Clone first to retain ownership.
    #[inline]
    pub fn downgrade(self) -> WeakRef<T> {
        self.0.inc_weak();
        WeakRef(self.0.clone())
    }

    /// Consumes this handle and checks the instance's runtime class before casting.
    /// Returns `None` for a null handle or if the instance is not `U` or a subclass
    /// of `U`. A failed cast drops this handle.
    pub fn cast<U>(self) -> Option<Ref<U>>
    where
        U: ScriptClass,
    {
        let inst = unsafe { (self.0.0.instance as *const ISerializable).as_ref() }?;
        inst.is_a::<U>().then(|| unsafe { mem::transmute(self) })
    }

    /// Returns whether the reference is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.0.instance.is_null()
    }

    /// Returns whether the runtime class is exactly `U`; false for a null handle.
    #[inline]
    pub fn is_exactly_a<U>(&self) -> bool
    where
        U: ScriptClass,
    {
        unsafe { (self.0.0.instance as *const ISerializable).as_ref() }
            .is_some_and(ISerializable::is_exactly_a::<U>)
    }

    /// Returns whether the instance is `U` or a subclass of `U`; false if null.
    #[inline]
    pub fn is_a<U>(&self) -> bool
    where
        U: ScriptClass,
    {
        unsafe { (self.0.0.instance as *const ISerializable).as_ref() }
            .is_some_and(ISerializable::is_a::<U>)
    }
}

impl<T: ScriptClass> Default for Ref<T> {
    #[inline]
    fn default() -> Self {
        Self(BaseRef::default())
    }
}

impl<T: ScriptClass> Clone for Ref<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.0.inc_strong();
        Self(self.0.clone())
    }
}

impl<T: ScriptClass> PtrEq for Ref<T> {
    #[inline]
    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl<T: ScriptClass> PtrEq<WeakRef<T>> for Ref<T> {
    #[inline]
    fn ptr_eq(&self, other: &WeakRef<T>) -> bool {
        self.0.0.ptr_eq(&other.0.0)
    }
}

impl<T: ScriptClass> Drop for Ref<T> {
    #[inline]
    fn drop(&mut self) {
        if self.0.dec_strong() && !self.0.0.instance.is_null() {
            let ptr = self.0.0.instance.cast::<NativeType<T>>();
            unsafe { ptr::drop_in_place(ptr) }
        }
    }
}

unsafe impl<T: ScriptClass> Send for Ref<T> {}
unsafe impl<T: ScriptClass> Sync for Ref<T> {}

/// A weak handle to a class instance (`wref<T>` in REDscript).
///
/// Does not keep the instance alive. [`Self::upgrade`] obtains a strong [`Ref`]
/// only while the instance still exists. Cloning copies the weak handle;
/// a default weak handle is empty.
///
/// SDK: [WeakHandle](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Handle.hpp).
#[repr(transparent)]
pub struct WeakRef<T: ScriptClass>(BaseRef<NativeType<T>>);

impl<T: ScriptClass> WeakRef<T> {
    /// Consumes this weak handle and obtains a strong handle if the instance is alive.
    /// Returns `None` for an empty handle or an instance with no strong handles.
    /// Clone first if you want to keep the weak handle.
    #[inline]
    pub fn upgrade(self) -> Option<Ref<T>> {
        self.0.inc_strong_if_non_zero().then(|| Ref(self.0.clone()))
    }
}

impl<T: ScriptClass> Default for WeakRef<T> {
    #[inline]
    fn default() -> Self {
        Self(BaseRef::default())
    }
}

impl<T: ScriptClass> Clone for WeakRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.0.inc_weak();
        Self(self.0.clone())
    }
}

impl<T: ScriptClass> Drop for WeakRef<T> {
    #[inline]
    fn drop(&mut self) {
        self.0.dec_weak();
    }
}

impl<T: ScriptClass> PtrEq for WeakRef<T> {
    #[inline]
    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl<T: ScriptClass> PtrEq<Ref<T>> for WeakRef<T> {
    #[inline]
    fn ptr_eq(&self, other: &Ref<T>) -> bool {
        self.0.0.ptr_eq(&other.0.0)
    }
}

unsafe impl<T: ScriptClass> Send for WeakRef<T> {}
unsafe impl<T: ScriptClass> Sync for WeakRef<T> {}

#[derive(Debug)]
#[repr(transparent)]
struct BaseRef<T>(red::SharedPtrBase<T>);

impl<T> BaseRef<T> {
    #[inline]
    fn instance(&self) -> Option<&T> {
        unsafe { self.0.instance.as_ref() }
    }

    #[inline]
    fn instance_mut(&mut self) -> Option<&mut T> {
        unsafe { self.0.instance.as_mut() }
    }

    #[inline]
    fn inc_strong(&self) {
        // Match the SDK's unsuffixed Windows Interlocked operations, including
        // their full barriers before promotion and final destruction.
        if let Some(cnt) = self.ref_count() {
            cnt.strong().fetch_add(1, Ordering::SeqCst);
        }
    }

    fn inc_strong_if_non_zero(&self) -> bool {
        let Some(cnt) = self.ref_count() else {
            return false;
        };

        cnt.strong()
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
                (x != 0).then(|| x + 1)
            })
            .is_ok()
    }

    fn dec_strong(&mut self) -> bool {
        let Some(cnt) = self.ref_count() else {
            return false;
        };

        if cnt.strong().fetch_sub(1, Ordering::SeqCst) == 1 {
            self.dec_weak();
            true
        } else {
            false
        }
    }

    #[inline]
    fn inc_weak(&self) {
        if let Some(cnt) = self.ref_count() {
            cnt.weak_refs().fetch_add(1, Ordering::SeqCst);
        }
    }

    fn dec_weak(&mut self) {
        if self.0.refCount.is_null() {
            return;
        }
        unsafe {
            let dec_weak = crate::fn_from_hash!(Handle_DecWeakRef, unsafe extern "C" fn(VoidPtr));
            dec_weak(self as *mut _ as VoidPtr);
        }
    }

    #[inline]
    fn ref_count(&self) -> Option<&RefCount> {
        unsafe { self.0.refCount.cast::<RefCount>().as_ref() }
    }
}

impl<T> Default for BaseRef<T> {
    #[inline]
    fn default() -> Self {
        Self(red::SharedPtrBase::default())
    }
}

impl<T> Clone for BaseRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(red::SharedPtrBase {
            instance: self.0.instance,
            refCount: self.0.refCount,
            ..Default::default()
        })
    }
}

impl<T> PtrEq for BaseRef<T> {
    #[inline]
    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct RefCount(red::RefCnt);

impl RefCount {
    #[inline]
    fn strong(&self) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr(&self.0.strongRefs as *const _ as _) }
    }

    #[inline]
    fn weak_refs(&self) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr(&self.0.weakRefs as *const _ as _) }
    }
}

#[cfg(test)]
mod count_tests {
    use super::*;

    fn with_count(strong: u32, weak: u32) -> (Box<red::RefCnt>, BaseRef<()>) {
        let mut count = Box::new(red::RefCnt {
            strongRefs: strong,
            weakRefs: weak,
        });
        let base = BaseRef(red::SharedPtrBase {
            instance: ptr::null_mut(),
            refCount: &mut *count,
            ..Default::default()
        });
        (count, base)
    }

    #[test]
    fn counts_match_sdk_transitions_without_final_release() {
        let (count, mut base) = with_count(1, 1);
        base.inc_strong();
        assert_eq!(count.strongRefs, 2);
        assert!(base.inc_strong_if_non_zero());
        assert_eq!(count.strongRefs, 3);
        assert!(!base.dec_strong());
        assert_eq!(count.strongRefs, 2);
        base.inc_weak();
        assert_eq!(count.weakRefs, 2);
    }

    #[test]
    fn promotion_never_revives_dead_or_empty_counts() {
        let (count, base) = with_count(0, 1);
        assert!(!base.inc_strong_if_non_zero());
        assert_eq!(count.strongRefs, 0);
        assert!(!BaseRef::<()>::default().inc_strong_if_non_zero());
    }
}

/// A typed borrow of an existing value (`script_ref<T>` in REDscript).
///
/// Used to pass a value by reference to script functions. Unlike [`Ref`], it
/// neither allocates an instance nor uses reference counting. The original
/// value stays borrowed for `'a` and must not be retained by the called script.
///
/// SDK: [ScriptRef](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct ScriptRef<'a, T>(red::ScriptRef<T>, PhantomData<&'a mut T>);

impl<'a, T: NativeRepr> ScriptRef<'a, T> {
    /// Borrows a value and records its RTTI type.
    /// Returns `None` if `T` is not registered; requires initialized engine RTTI.
    pub fn new(val: &'a mut T) -> Option<Self> {
        let rtti = RttiSystem::get();
        let inner = rtti.get_type(CName::new(T::NAME))?;
        let ref_ = red::ScriptRef {
            innerType: inner.as_raw() as *const _ as *mut red::rtti::IType,
            ref_: val as *mut T,
            ..Default::default()
        };
        Some(Self(ref_, PhantomData))
    }

    /// Borrows the referenced value, or returns `None` if the reference is undefined.
    #[inline]
    pub fn value(&self) -> Option<&T> {
        unsafe { self.0.ref_.as_ref() }
    }

    /// Returns the type of the value being referenced.
    #[inline]
    pub fn inner_type(&self) -> &Type {
        unsafe { &*(self.0.innerType.cast::<Type>()) }
    }

    /// Returns whether the reference is defined.
    #[inline]
    pub fn is_defined(&self) -> bool {
        !self.0.ref_.is_null()
    }
}
