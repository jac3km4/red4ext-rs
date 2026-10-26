use std::num::NonZeroUsize;
use std::{mem, ops, ptr};

use once_cell::race::OnceNonZeroUsize;
use sealed::sealed;

use super::{GlobalFunction, IScriptable, Method, Property, RwSpinLockReadGuard, StaticMethod};
use crate::raw::root::RED4ext as red;
use crate::raw::root::RED4ext::Memory::AllocationResult;
use crate::{VoidPtr, fnv1a32};

/// Engine allocator interface.
///
/// SDK: [allocators](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Memory/Allocators.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct IAllocator(red::Memory::IAllocator);

impl IAllocator {
    /// Releases storage without dropping its contents.
    ///
    /// # Safety
    /// `memory` must be a live allocation from this allocator, with any required
    /// payload destruction already complete. It must not be used or freed afterward.
    #[inline]
    pub unsafe fn free<T>(&self, memory: *mut T) {
        let mut alloc = AllocationResult {
            memory: memory as VoidPtr,
            size: 0,
        };
        unsafe {
            ((*self.0.vtable_).IAllocator_Free)(
                &self.0 as *const _ as *mut red::Memory::IAllocator,
                &mut alloc,
            )
        }
    }

    /// Allocates `size` bytes with the requested alignment; returns null on failure.
    ///
    /// # Safety
    /// Requires initialized engine memory systems and a size and alignment valid
    /// for this allocator. Before accessing the result as `T`, ensure sufficient
    /// size and alignment and initialize it. Release it through the same allocator.
    #[inline]
    pub unsafe fn alloc_aligned<T>(&self, size: u32, alignment: u32) -> *mut T {
        unsafe {
            let result = ((*self.0.vtable_).IAllocator_GetHandle)(
                &self.0 as *const _ as *mut red::Memory::IAllocator,
            );
            let vault = vault_get(result);
            vault_alloc_aligned(vault, size, alignment).unwrap_or(ptr::null_mut()) as _
        }
    }
}

/// Owns a value allocated from an engine memory pool.
///
/// Drop destroys `T` and releases storage through its pool, requiring initialized
/// engine memory systems. `PoolRef<MaybeUninit<T>>` releases only storage.
///
/// SDK: [memory pool](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Memory/Pool.hpp).
#[derive(Debug)]
pub struct PoolRef<T: Poolable>(*mut T);

impl<T: Poolable> PoolRef<mem::MaybeUninit<T>> {
    #[inline]
    pub(super) unsafe fn assume_init(self) -> PoolRef<T> {
        let res = PoolRef(self.0 as *mut T);
        mem::forget(self);
        res
    }
}

impl<T: Poolable> ops::Deref for PoolRef<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T: Poolable> ops::DerefMut for PoolRef<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

impl<T: Poolable> Drop for PoolRef<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(self.0);
            pool_free(self.0);
        }
    }
}

/// Associates an engine-allocated type with its SDK memory pool.
#[sealed]
pub trait Poolable {
    /// Pool used to allocate and release this type.
    type Pool: Pool;
}

#[sealed]
impl Poolable for GlobalFunction {
    type Pool = FunctionPool;
}

#[sealed]
impl Poolable for Method {
    type Pool = FunctionPool;
}

#[sealed]
impl Poolable for StaticMethod {
    type Pool = FunctionPool;
}

#[sealed]
impl Poolable for Property {
    type Pool = PropertyPool;
}

#[sealed]
impl Poolable for IScriptable {
    type Pool = ScriptPool;
}

#[sealed]
impl<T> Poolable for mem::MaybeUninit<T>
where
    T: Poolable,
{
    type Pool = T::Pool;
}

/// Allocation operations for pool-backed types.
#[sealed]
pub trait PoolableOps: Poolable + Sized {
    /// Allocates uninitialized storage for `Self`, returning `None` on allocation failure.
    ///
    /// Requires initialized engine memory systems. Dropping the result releases
    /// storage without destroying an uninitialized value.
    fn alloc() -> Option<PoolRef<mem::MaybeUninit<Self>>>;
}

#[sealed]
impl<T: Poolable> PoolableOps for T {
    fn alloc() -> Option<PoolRef<mem::MaybeUninit<Self>>> {
        let result = unsafe { vault_alloc(T::Pool::vault(), mem::size_of::<T>() as u32)? };
        (!result.is_null()).then(|| PoolRef(result.cast::<mem::MaybeUninit<Self>>()))
    }
}

unsafe fn pool_free<T: Poolable>(memory: *mut T) {
    let mut alloc = AllocationResult {
        memory: memory as VoidPtr,
        size: 0,
    };
    unsafe {
        let free = crate::fn_from_hash!(
            Memory_Vault_Free,
            unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult)
        );
        free(T::Pool::vault(), &mut alloc);
    }
}

/// Identifies an SDK memory pool.
#[sealed]
pub trait Pool {
    /// Engine name used to locate this pool.
    const NAME: &'static str;

    #[doc(hidden)]
    fn vault_cache() -> &'static OnceNonZeroUsize;

    /// Returns the pool's cached native vault.
    ///
    /// Requires initialized engine memory systems. Panics if the pool cannot be resolved.
    fn vault() -> *mut red::Memory::Vault {
        Self::vault_cache()
            .get_or_try_init(|| {
                NonZeroUsize::new(unsafe { vault_get(fnv1a32(Self::NAME)) as _ }).ok_or(())
            })
            .expect("should resolve vault")
            .get() as _
    }
}

/// SDK pool for RTTI function metadata.
#[derive(Debug)]
pub struct FunctionPool;

#[sealed]
impl Pool for FunctionPool {
    const NAME: &'static str = "PoolRTTIFunction";

    fn vault_cache() -> &'static OnceNonZeroUsize {
        static VAULT: OnceNonZeroUsize = OnceNonZeroUsize::new();
        &VAULT
    }
}

/// SDK pool for RTTI property metadata.
#[derive(Debug)]
pub struct PropertyPool;

#[sealed]
impl Pool for PropertyPool {
    const NAME: &'static str = "PoolRTTIProperty";

    fn vault_cache() -> &'static OnceNonZeroUsize {
        static VAULT: OnceNonZeroUsize = OnceNonZeroUsize::new();
        &VAULT
    }
}

/// SDK pool for script values.
#[derive(Debug)]
pub struct ScriptPool;

#[sealed]
impl Pool for ScriptPool {
    const NAME: &'static str = "PoolScript";

    fn vault_cache() -> &'static OnceNonZeroUsize {
        static VAULT: OnceNonZeroUsize = OnceNonZeroUsize::new();
        &VAULT
    }
}

pub(super) unsafe fn vault_alloc(vault: *mut red::Memory::Vault, size: u32) -> Option<VoidPtr> {
    let mut result = AllocationResult::default();
    unsafe {
        let alloc = crate::fn_from_hash!(
            Memory_Vault_Alloc,
            unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult, u32)
        );
        alloc(vault, &mut result, size as _);
    };
    (!result.memory.is_null()).then_some(result.memory)
}

pub(super) unsafe fn vault_alloc_aligned(
    vault: *mut red::Memory::Vault,
    size: u32,
    alignment: u32,
) -> Option<VoidPtr> {
    let mut result = AllocationResult::default();
    unsafe {
        let alloc_aligned = crate::fn_from_hash!(
            Memory_Vault_AllocAligned,
            unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult, u32, u32)
        );
        alloc_aligned(vault, &mut result, size as _, alignment as _);
    };
    (!result.memory.is_null()).then_some(result.memory)
}

#[cold]
pub(super) unsafe fn vault_get(handle: u32) -> *mut red::Memory::Vault {
    unsafe {
        let vault = red::Memory::Vault::Get();
        vault_get_from_registry(&raw mut (*vault).poolRegistry, handle)
    }
}

unsafe fn vault_get_from_registry(
    registry: *mut red::Memory::PoolRegistry,
    handle: u32,
) -> *mut red::Memory::Vault {
    unsafe {
        let nodes = RwSpinLockReadGuard::new(
            ptr::NonNull::new_unchecked(&raw mut (*registry).nodesLock),
            ptr::NonNull::new_unchecked(&raw mut (*registry).nodes),
        );
        let Some(info) = nodes.iter().find(|node| node.handle == handle) else {
            return ptr::null_mut();
        };
        ((*info.storage).allocatorStorage & !7) as _
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_caches_are_per_pool() {
        let caches = [
            FunctionPool::vault_cache(),
            PropertyPool::vault_cache(),
            ScriptPool::vault_cache(),
        ];
        for (i, cache) in caches.iter().enumerate() {
            for other in &caches[i + 1..] {
                assert!(!ptr::eq(*cache, *other));
            }
        }
        assert!(ptr::eq(FunctionPool::vault_cache(), caches[0]));
    }

    #[test]
    fn missing_pool_releases_registry_lock() {
        let mut registry = red::Memory::PoolRegistry::default();

        for _ in 0..2 {
            assert!(unsafe { vault_get_from_registry(&raw mut registry, 1) }.is_null());
            assert_eq!(registry.nodesLock.state, 0);
        }
    }

    #[test]
    fn matching_pool_masks_tags_and_releases_registry_lock() {
        let mut registry = red::Memory::PoolRegistry::default();
        let mut vault = mem::MaybeUninit::<red::Memory::Vault>::uninit();
        let expected = vault.as_mut_ptr();
        registry.nodes[0].handle = 1;

        for tag in 0..8 {
            let mut storage = red::Memory::PoolStorage {
                allocatorStorage: expected as u64 | tag,
                ..Default::default()
            };
            registry.nodes[0].storage = &mut storage;
            assert_eq!(
                unsafe { vault_get_from_registry(&raw mut registry, 1) },
                expected
            );
            assert_eq!(registry.nodesLock.state, 0);
        }
    }
}
