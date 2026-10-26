use std::num::NonZeroUsize;
use std::{mem, ops, ptr};

use once_cell::race::OnceNonZeroUsize;
use sealed::sealed;

use super::{GlobalFunction, IScriptable, Method, Property, RwSpinLockReadGuard, StaticMethod};
use crate::raw::root::RED4ext as red;
use crate::raw::root::RED4ext::Memory::AllocationResult;
use crate::{VoidPtr, fnv1a32};

/// Engine allocator interface; see the SDK's
/// [allocators](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Memory/Allocators.hpp).
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
    /// The size and alignment must be valid for the engine allocator. Initialize
    /// the allocation before reading it and release it through the same allocator.
    #[cfg(not(target_os = "macos"))]
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

    /// Allocates `size` bytes with the requested alignment; returns null on failure.
    ///
    /// # Safety
    /// The size and alignment must be valid for the engine allocator. Initialize
    /// the allocation before reading it and release it through the same allocator.
    ///
    /// macOS: asks the allocator itself (vtable slot 0x08, AllocAligned) instead of looking its handle up in the vault. The game's
    /// `GetHandle` (slot 0x30) returns `PoolStorage::allocatorhandle` of its own pool storage, which the vault stand-in
    /// (`deps/macos/MacVault.cpp`) does not mirror, so the Windows lookup by handle cannot be trusted. Evidence: Mac
    /// Allocator<PoolScript> vtable (vptr 0x71B58E8, reached from the CClass GetAllocator thunk 0x94C958 -> 0x344DBC4): slot 0x08 is the
    /// game's AllocateAligned wrapper and slot 0x30 is `adrp x8,..; ldr x8,[x8,#0xea8]; ldr w0,[x8,#0x20]; ret`. The `AllocationResult`
    /// is a 16 byte trivially copyable struct, returned in x0/x1 on arm64, which is how Rust's `extern "C"` reads it too.
    #[cfg(target_os = "macos")]
    #[inline]
    pub unsafe fn alloc_aligned<T>(&self, size: u32, alignment: u32) -> *mut T {
        unsafe {
            let result = ((*self.0.vtable_).IAllocator_AllocAligned)(
                &self.0 as *const _ as *const red::Memory::IAllocator,
                size as u64,
                alignment,
            );
            result.memory as _
        }
    }
}

/// Owns storage from an SDK
/// [memory pool](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Memory/Pool.hpp).
/// Drop destroys `T` and releases storage. `PoolRef<MaybeUninit<T>>` releases only storage.
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

/// A trait for types that can be stored in a pool.
#[sealed]
pub trait Poolable {
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

/// A trait with operations for types that can be stored in a pool.
#[sealed]
pub trait PoolableOps: Poolable + Sized {
    /// Allocates memory for `Self`. The resulting value must be initialized before use.
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

/// A trait for different types of pools.
#[sealed]
pub trait Pool {
    const NAME: &'static str;

    #[doc(hidden)]
    fn vault_cache() -> &'static OnceNonZeroUsize;

    fn vault() -> *mut red::Memory::Vault {
        Self::vault_cache()
            .get_or_try_init(|| {
                NonZeroUsize::new(unsafe { vault_get(fnv1a32(Self::NAME)) as _ }).ok_or(())
            })
            .expect("should resolve vault")
            .get() as _
    }
}

/// A pool for functions.
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

/// A pool for properties.
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

/// A pool for scripts values.
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

/// The type of the size argument of the vault functions. macOS: the stand-ins (`deps/macos/MacVault.cpp`) take `uint64_t`, and a `u32`
/// argument leaves the upper half of x2 unspecified.
#[cfg(not(target_os = "macos"))]
type VaultSize = u32;
#[cfg(target_os = "macos")]
type VaultSize = usize;

pub(super) unsafe fn vault_alloc(vault: *mut red::Memory::Vault, size: u32) -> Option<VoidPtr> {
    let mut result = AllocationResult::default();
    unsafe {
        let alloc = crate::fn_from_hash!(
            Memory_Vault_Alloc,
            unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult, VaultSize)
        );
        alloc(vault, &mut result, size as _);
    };
    (!result.memory.is_null()).then_some(result.memory)
}

// macOS: not called (`IAllocator::alloc_aligned` uses the allocator's own AllocAligned slot there).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) unsafe fn vault_alloc_aligned(
    vault: *mut red::Memory::Vault,
    size: u32,
    alignment: u32,
) -> Option<VoidPtr> {
    let mut result = AllocationResult::default();
    unsafe {
        let alloc_aligned = crate::fn_from_hash!(
            Memory_Vault_AllocAligned,
            unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult, VaultSize, u32)
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
