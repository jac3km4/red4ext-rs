use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use crate::raw::root::RED4ext as red;

pub struct RwSpinLockReadGuard<'a, T> {
    lock: &'a red::SharedSpinLock,
    value: NonNull<T>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> RwSpinLockReadGuard<'a, T> {
    #[inline]
    pub(crate) unsafe fn new(lock: &'a red::SharedSpinLock, value: NonNull<T>) -> Self {
        unsafe { red::SharedSpinLock_LockShared(lock as *const _ as *mut red::SharedSpinLock) };
        Self {
            value,
            lock,
            phantom: PhantomData,
        }
    }
}

impl<T> Deref for RwSpinLockReadGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.value.as_ref() }
    }
}

impl<T> Drop for RwSpinLockReadGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            red::SharedSpinLock_UnlockShared(self.lock as *const _ as *mut red::SharedSpinLock)
        };
    }
}

pub struct RwSpinLockWriteGuard<'a, T> {
    lock: &'a red::SharedSpinLock,
    value: NonNull<T>,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> RwSpinLockWriteGuard<'a, T> {
    #[inline]
    pub(crate) unsafe fn new(lock: &'a red::SharedSpinLock, value: NonNull<T>) -> Self {
        unsafe { red::SharedSpinLock_Lock(lock as *const _ as *mut red::SharedSpinLock) };
        Self {
            value,
            lock,
            phantom: PhantomData,
        }
    }
}

impl<T> Deref for RwSpinLockWriteGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.value.as_ref() }
    }
}

impl<T> DerefMut for RwSpinLockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.value.as_ptr() }
    }
}

impl<T> Drop for RwSpinLockWriteGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { red::SharedSpinLock_Unlock(self.lock as *const _ as *mut red::SharedSpinLock) };
    }
}
