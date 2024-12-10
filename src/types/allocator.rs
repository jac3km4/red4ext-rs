use std::num::NonZeroUsize;
use std::{mem, ops, ptr};

use once_cell::race::OnceNonZeroUsize;
use sealed::sealed;

use super::{GlobalFunction, IScriptable, Method, Property, StaticMethod};
use crate::raw::root::RED4ext as red;
use crate::raw::root::RED4ext::Memory::AllocationResult;
use crate::{fnv1a32, VoidPtr};

/// An interface for allocating and freeing memory.
#[derive(Debug)]
#[repr(transparent)]
pub struct IAllocator(red::Memory::IAllocator);

impl IAllocator {
    /// Frees the memory pointed by `memory`.
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

    /// Allocates `size` bytes of memory with `alignment` bytes alignment.
    #[inline]
    pub unsafe fn alloc_aligned<T>(&self, size: u32, alignment: u32) -> *mut T {
        let result = unsafe {
            ((*self.0.vtable_).IAllocator_GetHandle)(
                &self.0 as *const _ as *mut red::Memory::IAllocator,
            )
        };
        let vault = vault_get(result);
        vault_alloc_aligned(vault, size, alignment).unwrap_or(ptr::null_mut()) as _
    }
}

/// A reference to a value stored in a pool.
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
        unsafe { ptr::drop_in_place(self.0) };
        T::free(self);
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
    /// Frees memory pointed by `ptr`.
    fn free(ptr: &mut PoolRef<Self>);
}

#[sealed]
impl<T: Poolable> PoolableOps for T {
    fn alloc() -> Option<PoolRef<mem::MaybeUninit<Self>>> {
        let result = unsafe { vault_alloc(T::Pool::vault(), mem::size_of::<T>() as u32)? };
        (!result.is_null()).then(|| PoolRef(result.cast::<mem::MaybeUninit<Self>>()))
    }

    fn free(ptr: &mut PoolRef<Self>) {
        let mut alloc = AllocationResult {
            memory: ptr.0 as VoidPtr,
            size: 0,
        };
        unsafe {
            let free = crate::fn_from_hash!(
                Memory_Vault_Free,
                unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult)
            );
            free(T::Pool::vault(), &mut alloc);
        };
    }
}

/// A trait for different types of pools.
#[sealed]
pub trait Pool {
    const NAME: &'static str;

    fn vault() -> *mut red::Memory::Vault {
        static VAULT: OnceNonZeroUsize = OnceNonZeroUsize::new();
        VAULT
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
}

/// A pool for properties.
#[derive(Debug)]
pub struct PropertyPool;

#[sealed]
impl Pool for PropertyPool {
    const NAME: &'static str = "PoolRTTIProperty";
}

/// A pool for RTTI.
#[derive(Debug)]
pub struct RttiPool;

#[sealed]
impl Pool for RttiPool {
    const NAME: &'static str = "PoolRTTI";
}

/// A pool for scripts values.
#[derive(Debug)]
pub struct ScriptPool;

#[sealed]
impl Pool for ScriptPool {
    const NAME: &'static str = "PoolScript";
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
    let vault = &mut *red::Memory::Vault::Get();

    vault.poolRegistry.nodesLock.lock_shared();
    let Some(info) = vault
        .poolRegistry
        .nodes
        .iter()
        .find(|node| node.handle == handle)
    else {
        return ptr::null_mut();
    };
    let storage = (*info.storage).allocatorStorage & !7;
    vault.poolRegistry.nodesLock.unlock_shared();

    storage as _
}
