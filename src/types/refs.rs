use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{mem, ptr};

use super::{CName, ISerializable, PoolRef, Type};
use crate::class::{NativeType, ScriptClass};
use crate::raw::root::RED4ext as red;
use crate::repr::NativeRepr;
use crate::systems::RttiSystem;
use crate::types::PoolableOps;
use crate::{ClassKind, VoidPtr};

/// A reference counted shared pointer to a script class.
#[repr(transparent)]
pub struct Ref<T: ScriptClass>(BaseRef<NativeType<T>>);

impl<T: ScriptClass> Ref<T> {
    /// Creates a new reference to the class.
    #[inline]
    pub fn new() -> Option<Self> {
        Self::new_with(|_| {})
    }

    /// Creates a new reference to the class and initializes it with the provided function.
    pub fn new_with(init: impl FnOnce(&mut T)) -> Option<Self> {
        let system = RttiSystem::get();
        let class = system.get_class(CName::new(T::NAME))?;
        let mut this = Self::default();
        Self::ctor(&mut this, class.instantiate().as_ptr().cast::<T>());

        init(T::Kind::fields_mut(this.0.instance_mut()?));
        Some(this)
    }

    fn ctor(this: *mut Self, data: *mut T) {
        unsafe {
            let ctor = crate::fn_from_hash!(Handle_ctor, unsafe extern "C" fn(VoidPtr, VoidPtr));
            ctor(this as *mut _ as VoidPtr, data as *mut _ as VoidPtr);
        }
    }

    /// Returns a reference to the fields of the class.
    ///
    /// # Safety
    /// The underlying value can be accessed mutably at any point through another copy of the
    /// [`Ref`]. Ideally, the caller should ensure that the returned reference is short-lived.
    #[inline]
    pub unsafe fn fields(&self) -> Option<&T> {
        Some(T::Kind::fields(self.0.instance()?))
    }

    /// Returns a mutable reference to the fields of the class.
    ///
    /// # Safety
    /// The underlying value can be accessed mutably at any point through another copy of the
    /// [`Ref`]. Ideally, the caller should ensure that the returned reference is short-lived.
    #[inline]
    pub unsafe fn fields_mut(&mut self) -> Option<&mut T> {
        Some(T::Kind::fields_mut(self.0.instance_mut()?))
    }

    /// Returns a reference to the instance of the class.
    ///
    /// # Safety
    /// The underlying value can be accessed mutably at any point through another copy of the
    /// [`Ref`]. Ideally, the caller should ensure that the returned reference is short-lived.
    #[inline]
    pub unsafe fn instance(&self) -> Option<&NativeType<T>> {
        self.0.instance()
    }

    /// Converts the reference to a [`WeakRef`]. This will decrement the strong reference count
    /// and increment the weak reference count.
    #[inline]
    pub fn downgrade(self) -> WeakRef<T> {
        self.0.inc_weak();
        WeakRef(self.0.clone())
    }

    /// Attempts to cast the reference to a reference of another class.
    /// Returns [`None`] if the target class is not compatible.
    pub fn cast<U>(self) -> Option<Ref<U>>
    where
        U: ScriptClass,
    {
        let inst = unsafe { (self.0 .0.instance as *const ISerializable).as_ref() }?;
        inst.is_a::<U>().then(|| unsafe { mem::transmute(self) })
    }

    /// Returns whether the reference is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 .0.instance.is_null()
    }

    #[inline]
    pub fn is_exactly_a<U>(&self) -> bool
    where
        U: ScriptClass,
    {
        unsafe { (self.0 .0.instance as *const ISerializable).as_ref() }
            .is_some_and(ISerializable::is_exactly_a::<U>)
    }

    #[inline]
    pub fn is_a<U>(&self) -> bool
    where
        U: ScriptClass,
    {
        unsafe { (self.0 .0.instance as *const ISerializable).as_ref() }
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

impl<T: ScriptClass> Drop for Ref<T> {
    #[inline]
    fn drop(&mut self) {
        if self.0.dec_strong() && !self.0 .0.instance.is_null() {
            let ptr = self.0 .0.instance.cast::<NativeType<T>>();
            unsafe { ptr::drop_in_place(ptr) }
        }
    }
}

unsafe impl<T: ScriptClass> Send for Ref<T> {}
unsafe impl<T: ScriptClass> Sync for Ref<T> {}

/// A weak reference to a script class.
/// Before use, it must be upgraded to a strong reference using [`WeakRef::upgrade`].
#[repr(transparent)]
pub struct WeakRef<T: ScriptClass>(BaseRef<NativeType<T>>);

impl<T: ScriptClass> WeakRef<T> {
    /// Attempts to upgrade the weak reference to a strong reference.
    /// Returns [`None`] if the strong reference count is zero.
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
        if let Some(cnt) = self.ref_count() {
            cnt.strong().fetch_add(1, Ordering::Relaxed);
        }
    }

    fn inc_strong_if_non_zero(&self) -> bool {
        let Some(cnt) = self.ref_count() else {
            return false;
        };

        cnt.strong()
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                (x != 0).then(|| x + 1)
            })
            .is_ok()
    }

    fn dec_strong(&mut self) -> bool {
        let Some(cnt) = self.ref_count() else {
            return false;
        };

        if cnt.strong().fetch_sub(1, Ordering::Relaxed) == 1 {
            self.dec_weak();
            true
        } else {
            false
        }
    }

    #[inline]
    fn inc_weak(&self) {
        if let Some(cnt) = self.ref_count() {
            cnt.weak_refs().fetch_add(1, Ordering::Relaxed);
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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct RefCount(red::RefCnt);

impl RefCount {
    #[inline]
    fn strong(&self) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr(&self.0.strongRefs as *const _ as _) }
    }

    #[inline]
    fn weak_refs(&self) -> &AtomicU32 {
        unsafe { AtomicU32::from_ptr(&self.0.weakRefs as *const _ as _) }
    }

    fn new() -> PoolRef<Self> {
        let mut refcount = RefCount::alloc().expect("should allocate a RefCount");
        let ptr = refcount.as_mut_ptr();
        unsafe {
            (*ptr).0.strongRefs = 1;
            (*ptr).0.weakRefs = 1;
            refcount.assume_init()
        }
    }
}

/// A reference to local script data.
#[derive(Debug)]
#[repr(transparent)]
pub struct ScriptRef<'a, T>(red::ScriptRef<T>, PhantomData<&'a mut T>);

impl<'a, T: NativeRepr> ScriptRef<'a, T> {
    /// Creates a new reference pointing to the provided value.
    pub fn new(val: &'a mut T) -> Option<Self> {
        let rtti = RttiSystem::get();
        let inner = rtti.get_type(CName::new(T::NAME))?;
        let ref_ = red::ScriptRef {
            innerType: inner.as_raw() as *const _ as *mut red::CBaseRTTIType,
            ref_: val as *mut T,
            ..Default::default()
        };
        Some(Self(ref_, PhantomData))
    }

    /// Returns the value being referenced.
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

#[derive(Debug)]
#[repr(transparent)]
pub struct SharedPtr<T>(red::SharedPtrBase<T>);

impl<T: NativeRepr> Clone for SharedPtr<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.inc_strong();
        unsafe { ptr::read(self) }
    }
}

impl<T: Default + NativeRepr> SharedPtr<T> {
    #[must_use]
    pub fn new_with(value: T) -> Self {
        let mut this = red::SharedPtrBase::<T>::default();
        let refcount = RefCount::new();
        this.refCount = refcount.0 as *mut red::RefCnt;
        this.instance = Box::leak(Box::new(value)) as *const _ as *mut _;
        mem::forget(refcount);
        Self(this)
    }
}

impl<T> SharedPtr<T> {
    #[inline]
    fn ref_count(&self) -> Option<&RefCount> {
        unsafe { self.0.refCount.cast::<RefCount>().as_ref() }
    }

    #[inline]
    fn inc_strong(&self) {
        if let Some(cnt) = self.ref_count() {
            cnt.strong().fetch_add(1, Ordering::Relaxed);
        }
    }

    fn dec_strong(&mut self) -> bool {
        let Some(cnt) = self.ref_count() else {
            return false;
        };

        cnt.strong().fetch_sub(1, Ordering::Relaxed) == 1
    }
}

impl<T> Drop for SharedPtr<T> {
    fn drop(&mut self) {
        if self.dec_strong() && !self.0.instance.is_null() {
            let own_refcount =
                unsafe { mem::transmute::<*mut red::RefCnt, PoolRef<RefCount>>(self.0.refCount) };
            let ptr_instance = self.0.instance;
            mem::drop(own_refcount);
            unsafe {
                ptr::drop_in_place(ptr_instance);
            }
        }
    }
}
