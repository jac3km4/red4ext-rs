use std::sync::atomic::{AtomicU32, Ordering};

use super::IAllocator;
use crate::VoidPtr;
use crate::raw::root::RED4ext as red;

#[derive(Debug)]
#[repr(transparent)]
/// Opaque data buffer.
///
/// Drop releases raw storage through its native allocator, if present.
///
/// SDK: [DataBuffer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Buffer.hpp).
pub struct DataBuffer(red::DataBuffer);

impl Drop for DataBuffer {
    fn drop(&mut self) {
        // DataBuffer's implicit SDK destructor destroys its RawBuffer member only.
        unsafe { self.0.buffer.destruct() }
    }
}

#[derive(Debug)]
#[repr(transparent)]
/// Opaque deferred data buffer.
///
/// Drop releases temporary storage and the shared reference to loaded data.
/// This wrapper does not expose the SDK's asynchronous loading operations.
///
/// SDK: [DeferredDataBuffer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Buffer.hpp).
pub struct DeferredDataBuffer(red::DeferredDataBuffer);

impl Drop for DeferredDataBuffer {
    fn drop(&mut self) {
        // C++ members are destroyed in reverse declaration order: raw, then temp.
        unsafe {
            release_raw_buffer(&mut self.0.raw._base._base, |shared| {
                let dec_weak =
                    crate::fn_from_hash!(Handle_DecWeakRef, unsafe extern "C" fn(VoidPtr));
                dec_weak(std::ptr::from_mut(shared).cast());
            });
            self.0.temp.destruct();
        }
    }
}

// Mirrors SharedPtr::~SharedPtr, SharedPtrBase::DecRef/Destroy and Memory::Delete.
// The weak-release callback may free the control block, but must preserve instance.
unsafe fn release_raw_buffer(
    shared: &mut red::SharedPtrBase<red::RawBuffer>,
    dec_weak: impl FnOnce(&mut red::SharedPtrBase<red::RawBuffer>),
) {
    if shared.refCount.is_null() {
        return;
    }
    let strong = unsafe { AtomicU32::from_ptr(&raw mut (*shared.refCount).strongRefs) };
    if strong.fetch_sub(1, Ordering::SeqCst) != 1 {
        return;
    }
    dec_weak(shared);
    unsafe {
        // Memory::Delete captures the dynamic allocator before destroying the buffer.
        let allocator = (*shared.instance).GetAllocator().cast::<IAllocator>();
        (*shared.instance).destruct();
        (*allocator).free(shared.instance);
    }
    shared.instance = std::ptr::null_mut();
    shared.refCount = std::ptr::null_mut();
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
/// Opaque, copyable shared data buffer value.
///
/// No buffer access or ownership operations are exposed.
///
/// SDK: [SharedDataBuffer](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Buffer.hpp).
pub struct SharedDataBuffer(red::SharedDataBuffer);

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::ptr;

    use super::*;

    thread_local! {
        static FREES: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    }

    type NativeAllocator = red::Memory::IAllocator;
    type Allocation = red::Memory::AllocationResult;

    #[repr(C)]
    struct RawAllocatorVtable {
        realloc_aligned: unsafe extern "C" fn(*const (), VoidPtr, u32, u32) -> VoidPtr,
        free: unsafe extern "C" fn(*const (), VoidPtr),
        get_allocator: unsafe extern "C" fn(*const ()) -> *mut NativeAllocator,
    }

    unsafe extern "C" fn get_allocator(this: *const ()) -> *mut NativeAllocator {
        // RawBufferAllocator embeds an IAllocator (its vtable word), not a pointer
        // to an IAllocator, immediately after its own vtable word.
        unsafe { this.cast::<u64>().add(1).cast_mut().cast() }
    }

    unsafe extern "C" fn free_buffer(this: *const (), memory: VoidPtr) {
        let allocator = unsafe { get_allocator(this) };
        let mut allocation = Allocation { memory, size: 0 };
        unsafe { ((*(*allocator).vtable_).IAllocator_Free)(allocator, &mut allocation) };
    }

    unsafe extern "C" fn realloc_buffer(_: *const (), _: VoidPtr, _: u32, _: u32) -> VoidPtr {
        ptr::null_mut()
    }

    unsafe extern "C" fn alloc(_: *const NativeAllocator, _: u64) -> Allocation {
        Allocation::default()
    }

    unsafe extern "C" fn alloc_aligned(_: *const NativeAllocator, _: u64, _: u32) -> Allocation {
        Allocation::default()
    }

    unsafe extern "C" fn realloc(
        _: *const NativeAllocator,
        _: *mut Allocation,
        _: u64,
    ) -> Allocation {
        Allocation::default()
    }

    unsafe extern "C" fn realloc_aligned(
        _: *const NativeAllocator,
        _: *mut Allocation,
        _: u64,
        _: u32,
    ) -> Allocation {
        Allocation::default()
    }

    unsafe extern "C" fn free_allocation(_: *const NativeAllocator, allocation: *mut Allocation) {
        FREES.with(|frees| {
            frees
                .borrow_mut()
                .push(unsafe { (*allocation).memory } as usize)
        });
    }

    unsafe extern "C" fn sub_28(_: *const NativeAllocator, _: VoidPtr) {}

    unsafe extern "C" fn get_handle(_: *const NativeAllocator) -> u32 {
        0
    }

    static ALLOCATOR_VTABLE: red::Memory::IAllocator__bindgen_vtable =
        red::Memory::IAllocator__bindgen_vtable {
            IAllocator_Alloc: alloc,
            IAllocator_AllocAligned: alloc_aligned,
            IAllocator_Realloc: realloc,
            IAllocator_ReallocAligned: realloc_aligned,
            IAllocator_Free: free_allocation,
            IAllocator_sub_28: sub_28,
            IAllocator_GetHandle: get_handle,
        };

    fn owned_raw(data: VoidPtr) -> red::RawBuffer {
        static VTABLE: RawAllocatorVtable = RawAllocatorVtable {
            realloc_aligned: realloc_buffer,
            free: free_buffer,
            get_allocator,
        };
        red::RawBuffer {
            data,
            size: 1,
            alignment: 8,
            allocator: [
                ptr::from_ref(&VTABLE) as u64,
                ptr::from_ref(&ALLOCATOR_VTABLE) as u64,
            ],
        }
    }

    #[test]
    fn data_buffer_destroys_owned_raw_member() {
        FREES.with(|frees| frees.borrow_mut().clear());
        let mut byte = 0u8;
        let data = ptr::from_mut(&mut byte).cast();
        drop(DataBuffer(red::DataBuffer {
            buffer: owned_raw(data),
            unk20: 0,
        }));
        FREES.with(|frees| assert_eq!(*frees.borrow(), [data as usize]));
    }

    #[test]
    fn raw_buffer_without_allocator_does_not_free_borrowed_data() {
        FREES.with(|frees| frees.borrow_mut().clear());
        let mut byte = 0u8;
        let mut raw = owned_raw(ptr::from_mut(&mut byte).cast());
        raw.allocator = [0, 0];
        drop(DataBuffer(red::DataBuffer {
            buffer: raw,
            unk20: 0,
        }));
        FREES.with(|frees| assert!(frees.borrow().is_empty()));
    }

    #[test]
    fn deferred_buffer_destroys_temp_and_releases_nonlast_shared_owner() {
        FREES.with(|frees| frees.borrow_mut().clear());
        let mut byte = 0u8;
        let data = ptr::from_mut(&mut byte).cast();
        let mut count = red::RefCnt {
            strongRefs: 2,
            weakRefs: 1,
        };
        let mut raw = red::RawBuffer::default();
        let mut deferred = red::DeferredDataBuffer {
            temp: owned_raw(data),
            ..Default::default()
        };
        deferred.raw._base._base.instance = &mut raw;
        deferred.raw._base._base.refCount = &mut count;
        drop(DeferredDataBuffer(deferred));
        assert_eq!(count.strongRefs, 1);
        assert_eq!(count.weakRefs, 1);
        FREES.with(|frees| assert_eq!(*frees.borrow(), [data as usize]));
    }

    #[test]
    fn last_shared_owner_destroys_payload_then_frees_object() {
        FREES.with(|frees| frees.borrow_mut().clear());
        let mut byte = 0u8;
        let data = ptr::from_mut(&mut byte).cast();
        let mut raw = owned_raw(data);
        let allocator = unsafe { raw.GetAllocator() };
        assert_eq!(allocator, ptr::from_mut(&mut raw.allocator[1]).cast());
        assert_eq!(
            unsafe { (*allocator).vtable_ },
            ptr::from_ref(&ALLOCATOR_VTABLE)
        );
        let object = ptr::from_mut(&mut raw);
        let mut count = red::RefCnt {
            strongRefs: 1,
            weakRefs: 2,
        };
        let mut shared = red::SharedPtrBase {
            instance: object,
            refCount: &mut count,
            ..Default::default()
        };
        unsafe {
            release_raw_buffer(&mut shared, |shared| {
                assert_eq!((*shared.refCount).strongRefs, 0);
                (*shared.refCount).weakRefs -= 1;
            });
        }
        assert_eq!(count.strongRefs, 0);
        assert_eq!(count.weakRefs, 1);
        assert!(shared.instance.is_null());
        assert!(shared.refCount.is_null());
        FREES.with(|frees| assert_eq!(*frees.borrow(), [data as usize, object as usize]));
    }

    #[test]
    fn empty_native_members_need_no_engine() {
        drop(DataBuffer(red::DataBuffer::default()));
        drop(DeferredDataBuffer(red::DeferredDataBuffer::default()));
        let mut shared = red::SharedPtrBase::default();
        unsafe { release_raw_buffer(&mut shared, |_| panic!("empty weak release")) };
    }
}
