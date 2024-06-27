use std::num::NonZero;
use std::{mem, ops, ptr};

use once_cell::race::OnceNonZeroUsize;
use sealed::sealed;

use super::{GlobalFunction, IScriptable};
use crate::raw::root::RED4ext as red;
use crate::raw::root::RED4ext::Memory::AllocationResult;
use crate::{fnv1a32, VoidPtr};

#[derive(Debug)]
#[repr(transparent)]
pub struct IAllocator(red::Memory::IAllocator);

impl IAllocator {
    #[inline]
    pub unsafe fn free<T>(&mut self, memory: *mut T) {
        let mut alloc = AllocationResult {
            memory: memory as VoidPtr,
            size: 0,
        };
        unsafe { ((*self.0.vtable_).IAllocator_Free)(&mut self.0, &mut alloc) }
    }
}

#[derive(Debug)]
pub struct PoolRef<T: Poolable>(*mut T);

impl<T: Poolable> PoolRef<mem::MaybeUninit<T>> {
    #[inline]
    pub unsafe fn assume_init(self) -> PoolRef<T> {
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

pub trait Poolable {
    type Pool: Pool;
}

impl Poolable for GlobalFunction {
    type Pool = FunctionPool;
}

impl Poolable for IScriptable {
    type Pool = ScriptPool;
}

impl<T> Poolable for mem::MaybeUninit<T>
where
    T: Poolable,
{
    type Pool = T::Pool;
}

#[sealed]
pub trait PoolableOps: Poolable + Sized {
    fn alloc() -> Option<PoolRef<mem::MaybeUninit<Self>>>;
    fn free(ptr: &mut PoolRef<Self>);
}

#[sealed]
impl<T: Poolable> PoolableOps for T {
    fn alloc() -> Option<PoolRef<mem::MaybeUninit<Self>>> {
        let mut result = AllocationResult::default();
        let size = mem::size_of::<Self>();
        unsafe {
            let alloc = crate::fn_from_hash!(
                Memory_Vault_Alloc,
                unsafe extern "C" fn(*mut red::Memory::Vault, *mut AllocationResult, u32)
            );
            alloc(T::Pool::vault(), &mut result, size as _);
        };
        (!result.memory.is_null()).then(|| PoolRef(result.memory.cast::<mem::MaybeUninit<Self>>()))
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

#[sealed]
pub trait Pool {
    const NAME: &'static str;

    fn vault() -> *mut red::Memory::Vault {
        static VAULT: OnceNonZeroUsize = OnceNonZeroUsize::new();
        VAULT
            .get_or_try_init(|| unsafe { vault_get(fnv1a32(Self::NAME)) }.ok_or(()))
            .expect("should resolve vault")
            .get() as _
    }
}

#[derive(Debug)]
pub struct FunctionPool;

#[sealed]
impl Pool for FunctionPool {
    const NAME: &'static str = "PoolRTTIFunction";
}

#[derive(Debug)]
pub struct RttiPool;

#[sealed]
impl Pool for RttiPool {
    const NAME: &'static str = "PoolRTTI";
}

#[derive(Debug)]
pub struct ScriptPool;

#[sealed]
impl Pool for ScriptPool {
    const NAME: &'static str = "PoolScript";
}

// the vault is cached, so this function is called only once per pool, inlining is unproductive
#[inline(never)]
unsafe fn vault_get(handle: u32) -> Option<NonZero<usize>> {
    let vault = &mut *red::Memory::Vault::Get();

    vault.poolRegistry.nodesLock.lock_shared();
    let info = vault
        .poolRegistry
        .nodes
        .iter()
        .find(|node| node.handle == handle)?;
    let storage = (*info.storage).allocatorStorage & !7;
    vault.poolRegistry.nodesLock.unlock_shared();

    NonZero::new(storage as usize)
}
