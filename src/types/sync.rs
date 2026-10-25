use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use crate::raw::root::RED4ext as red;

/// Shared guard for SDK [SharedSpinLock](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/SharedSpinLock.hpp).
/// Releases the read lock on drop.
pub struct RwSpinLockReadGuard<'a, T> {
    lock: &'a red::SharedSpinLock,
    value: NonNull<T>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> RwSpinLockReadGuard<'a, T> {
    /// # Safety
    /// Both pointers must remain valid for `'a`. The lock must protect `value`,
    /// and all conflicting accesses must use it. Neither pointer may originate
    /// from a shared reference to memory that the native code mutates.
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

/// Exclusive guard for SDK [SharedSpinLock](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/SharedSpinLock.hpp).
/// Releases the write lock on drop.
pub struct RwSpinLockWriteGuard<'a, T> {
    lock: &'a red::SharedSpinLock,
    value: NonNull<T>,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> RwSpinLockWriteGuard<'a, T> {
    /// # Safety
    /// Both pointers must remain valid for `'a`. The lock must protect `value`,
    /// and all conflicting accesses must use it. Neither pointer may originate
    /// from a shared reference to memory that the native code mutates.
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
