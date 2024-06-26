use std::sync::atomic::{AtomicU32, Ordering};

use super::{Type, ValuePtr};
use crate::raw::root::RED4ext as red;

pub unsafe trait IsScriptable {}

#[derive(Debug)]
#[repr(transparent)]
pub struct Ref<T: IsScriptable>(BaseRef<T>);

impl<T: IsScriptable> Ref<T> {
    pub fn downgrade(self) -> WeakRef<T> {
        self.0.inc_weak();
        WeakRef(self.0.clone())
    }
}

impl<T: IsScriptable> Default for Ref<T> {
    #[inline]
    fn default() -> Self {
        Self(BaseRef::default())
    }
}

impl<T: IsScriptable> Clone for Ref<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.0.inc_strong();
        Self(self.0.clone())
    }
}

impl<T: IsScriptable> Drop for Ref<T> {
    #[inline]
    fn drop(&mut self) {
        if self.0.dec_strong() && !self.0 .0.instance.is_null() {
            let ptr = self.0 .0.instance.cast::<red::IScriptable>();
            unsafe { red::IScriptable_IScriptable_destructor(ptr) }
        }
    }
}

unsafe impl<T: IsScriptable> Send for Ref<T> {}
unsafe impl<T: IsScriptable> Sync for Ref<T> {}

#[derive(Debug)]
#[repr(transparent)]
pub struct WeakRef<T: IsScriptable>(BaseRef<T>);

impl<T: IsScriptable> WeakRef<T> {
    #[inline]
    pub fn upgrade(self) -> Option<Ref<T>> {
        self.0.inc_strong_if_non_zero().then(|| Ref(self.0.clone()))
    }
}

impl<T: IsScriptable> Default for WeakRef<T> {
    #[inline]
    fn default() -> Self {
        Self(BaseRef::default())
    }
}

impl<T: IsScriptable> Clone for WeakRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.0.inc_weak();
        Self(self.0.clone())
    }
}

impl<T: IsScriptable> Drop for WeakRef<T> {
    #[inline]
    fn drop(&mut self) {
        self.0.dec_weak();
    }
}

unsafe impl<T: IsScriptable> Send for WeakRef<T> {}
unsafe impl<T: IsScriptable> Sync for WeakRef<T> {}

#[derive(Debug)]
#[repr(transparent)]
struct BaseRef<T>(red::SharedPtrBase<T>);

impl<T> BaseRef<T> {
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
            let dec_weak =
                crate::fn_from_hash!(Handle_DecWeakRef, unsafe extern "C" fn(&mut BaseRef<T>));
            dec_weak(self);
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
pub struct ScriptRef<T>(red::ScriptRef<T>);

impl<T> ScriptRef<T> {
    #[inline]
    pub fn inner_type(&self) -> &Type {
        unsafe { &*(self.0.innerType.cast::<Type>()) }
    }

    #[inline]
    pub fn is_defined(&self) -> bool {
        !self.0.ref_.is_null()
    }
}

pub type ScriptRefAny = ScriptRef<std::os::raw::c_void>;

impl ScriptRefAny {
    #[inline]
    pub fn value(&self) -> ValuePtr {
        ValuePtr::new(self.0.ref_)
    }
}
