use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use crate::raw::root::RED4ext as red;

/// Shared guard for a native shared spinlock.
///
/// Dereferences to the protected value and releases the read lock on drop.
/// Release it before requesting a write lock on the same lock.
///
/// SDK: [SharedSpinLock](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/SharedSpinLock.hpp).
pub struct RwSpinLockReadGuard<'a, T> {
    lock: NonNull<red::SharedSpinLock>,
    value: NonNull<T>,
    phantom: PhantomData<&'a T>,
}

impl<'a, T> RwSpinLockReadGuard<'a, T> {
    /// # Safety
    /// Both pointers must remain valid for `'a`. The lock must protect `value`,
    /// and all conflicting accesses must use it. Neither pointer may originate
    /// from a shared reference to memory that the native code mutates.
    #[inline]
    pub(crate) unsafe fn new(lock: NonNull<red::SharedSpinLock>, value: NonNull<T>) -> Self {
        unsafe { red::SharedSpinLock_LockShared(lock.as_ptr()) };
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
        unsafe { red::SharedSpinLock_UnlockShared(self.lock.as_ptr()) };
    }
}

/// Exclusive guard for a native shared spinlock.
///
/// Provides mutable access to the protected value and releases the write lock on
/// drop. Release it before requesting any other guard on the same lock.
///
/// SDK: [SharedSpinLock](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/SharedSpinLock.hpp).
pub struct RwSpinLockWriteGuard<'a, T> {
    lock: NonNull<red::SharedSpinLock>,
    value: NonNull<T>,
    phantom: PhantomData<&'a mut T>,
}

impl<'a, T> RwSpinLockWriteGuard<'a, T> {
    /// # Safety
    /// Both pointers must remain valid for `'a`. The lock must protect `value`,
    /// and all conflicting accesses must use it. Neither pointer may originate
    /// from a shared reference to memory that the native code mutates.
    #[inline]
    pub(crate) unsafe fn new(lock: NonNull<red::SharedSpinLock>, value: NonNull<T>) -> Self {
        unsafe { red::SharedSpinLock_Lock(lock.as_ptr()) };
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
        unsafe { red::SharedSpinLock_Unlock(self.lock.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use std::cell::UnsafeCell;

    use super::*;

    #[test]
    fn readers_share_lock_and_release_on_drop() {
        let lock = UnsafeCell::new(red::SharedSpinLock::default());
        let value = UnsafeCell::new(42);
        unsafe {
            let lock = NonNull::new_unchecked(lock.get());
            let value = NonNull::new_unchecked(value.get());
            let first = RwSpinLockReadGuard::new(lock, value);
            let second = RwSpinLockReadGuard::new(lock, value);
            assert_eq!(*first, 42);
            assert_eq!(*second, 42);
            assert_eq!((*lock.as_ptr()).state, 2);
            drop(first);
            assert_eq!((*lock.as_ptr()).state, 1);
            drop(second);
            assert_eq!((*lock.as_ptr()).state, 0);
        }
    }

    #[test]
    fn writer_mutates_value_and_releases_on_drop() {
        let lock = UnsafeCell::new(red::SharedSpinLock::default());
        let value = UnsafeCell::new(0);
        unsafe {
            let lock = NonNull::new_unchecked(lock.get());
            let mut guard = RwSpinLockWriteGuard::new(lock, NonNull::new_unchecked(value.get()));
            *guard = 42;
            assert_eq!(*guard, 42);
            drop(guard);
            assert_eq!((*lock.as_ptr()).state, 0);
            assert_eq!(*value.get(), 42);
        }
    }
}
