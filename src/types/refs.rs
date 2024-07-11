use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{iter, mem, ptr};

use sealed::sealed;

use super::{CName, IScriptable, ISerializable, Type};
use crate::raw::root::RED4ext as red;
use crate::repr::NativeRepr;
use crate::systems::RttiSystem;
use crate::VoidPtr;

pub unsafe trait ScriptClass: Sized {
    type Kind: ClassKind<Self>;

    const CLASS_NAME: &'static str;
}

#[sealed]
pub trait ClassKind<T> {
    type NativeType;

    fn get(inst: &Self::NativeType) -> &T;
    fn get_mut(inst: &mut Self::NativeType) -> &mut T;
}

#[derive(Debug)]
pub struct Scripted;

#[sealed]
impl<T> ClassKind<T> for Scripted {
    type NativeType = IScriptable;

    #[inline]
    fn get(inst: &Self::NativeType) -> &T {
        unsafe { &*inst.fields().as_ptr().cast::<T>() }
    }

    #[inline]
    fn get_mut(inst: &mut Self::NativeType) -> &mut T {
        unsafe { &mut *inst.fields().as_ptr().cast::<T>() }
    }
}

#[derive(Debug)]
pub struct Native;

#[sealed]
impl<T> ClassKind<T> for Native {
    type NativeType = T;

    #[inline]
    fn get(inst: &Self::NativeType) -> &T {
        inst
    }

    #[inline]
    fn get_mut(inst: &mut Self::NativeType) -> &mut T {
        inst
    }
}

#[sealed]
pub trait ScriptClassOps: ScriptClass {
    fn new_ref() -> Option<Ref<Self>>;
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

type NativeType<T> = <<T as ScriptClass>::Kind as ClassKind<T>>::NativeType;

#[repr(transparent)]
pub struct Ref<T: ScriptClass>(BaseRef<NativeType<T>>);

impl<T: ScriptClass> Ref<T> {
    #[inline]
    pub fn new() -> Option<Self> {
        Self::new_with(|_| {})
    }

    pub fn new_with(init: impl FnOnce(&mut T)) -> Option<Self> {
        let system = RttiSystem::get();
        let class = system.get_class(CName::new(T::CLASS_NAME))?;
        let mut this = Self::default();
        Self::ctor(&mut this, class.instantiate().as_ptr().cast::<T>());

        init(T::Kind::get_mut(this.0.instance_mut()?));
        Some(this)
    }

    fn ctor(this: *mut Self, data: *mut T) {
        unsafe {
            let ctor = crate::fn_from_hash!(Handle_ctor, unsafe extern "C" fn(VoidPtr, VoidPtr));
            ctor(this as *mut _ as VoidPtr, data as *mut _ as VoidPtr);
        }
    }

    #[inline]
    pub unsafe fn fields(&self) -> Option<&T> {
        Some(T::Kind::get(self.0.instance()?))
    }

    #[inline]
    pub unsafe fn fields_mut(&mut self) -> Option<&mut T> {
        Some(T::Kind::get_mut(self.0.instance_mut()?))
    }

    #[inline]
    pub fn instance(&self) -> Option<&NativeType<T>> {
        self.0.instance()
    }

    #[inline]
    pub fn downgrade(self) -> WeakRef<T> {
        self.0.inc_weak();
        WeakRef(self.0.clone())
    }

    pub fn cast<U>(self) -> Option<Ref<U>>
    where
        U: ScriptClass,
    {
        let inst = unsafe { (self.0 .0.instance as *const ISerializable).as_ref() }?;
        let class = inst.class();
        iter::once(class)
            .chain(class.base_iter())
            .any(|class| class.name() == CName::new(U::CLASS_NAME))
            .then(|| unsafe { mem::transmute(self) })
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

#[repr(transparent)]
pub struct WeakRef<T: ScriptClass>(BaseRef<NativeType<T>>);

impl<T: ScriptClass> WeakRef<T> {
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
pub struct RefCount(red::RefCnt);

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

#[derive(Debug)]
#[repr(transparent)]
pub struct ScriptRef<'a, T>(red::ScriptRef<T>, PhantomData<&'a mut T>);

impl<'a, T: NativeRepr> ScriptRef<'a, T> {
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

    #[inline]
    pub fn value(&self) -> Option<&T> {
        unsafe { self.0.ref_.as_ref() }
    }

    #[inline]
    pub fn inner_type(&self) -> &Type {
        unsafe { &*(self.0.innerType.cast::<Type>()) }
    }

    #[inline]
    pub fn is_defined(&self) -> bool {
        !self.0.ref_.is_null()
    }
}
