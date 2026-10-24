use std::marker::PhantomData;
use std::{ffi, mem, ptr};

/// No-argument C callback used for RTTI registration.
pub type VoidFunctionPointerCallback =
    Callback<extern "C" fn(), CallbackHandler<extern "C" fn(), ffi::c_void, ()>>;

/// Inline callable using the SDK
/// [Callback](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Callback.hpp) layout.
#[repr(C)]
pub struct Callback<F, H: CallbackStorage + 'static> {
    callback: PhantomData<F>,
    buffer: [usize; 4],
    handler: &'static H,
}

impl<F: Callable<A, R> + 'static, A: 'static, R: 'static> Callback<F, CallbackHandler<F, A, R>> {
    const HANDLER: CallbackHandler<F, A, R> = CallbackHandler {
        invoke: F::invoke,
        copy: F::copy,
        move_: F::move_,
        destroy: F::destroy,
    };

    pub fn new(callback: F) -> Self {
        assert!(
            mem::size_of::<F>() <= 0x20,
            "callback exceeds inline storage"
        );
        assert!(
            mem::align_of::<F>() <= mem::align_of::<usize>(),
            "callback exceeds inline alignment"
        );
        let mut buffer = [0usize; 4];
        unsafe { ptr::write(buffer.as_mut_ptr().cast::<F>(), callback) };
        Self {
            callback: PhantomData,
            buffer,
            handler: &Self::HANDLER,
        }
    }
}

impl<F, H: CallbackStorage + 'static> Drop for Callback<F, H> {
    fn drop(&mut self) {
        unsafe {
            self.handler
                .destroy_storage(self.buffer.as_mut_ptr().cast())
        };
    }
}

/// Operations for an inline SDK callable.
///
/// # Safety
/// Implementations must accept the SDK calling convention. Copy and move must
/// initialize an uninitialized, aligned destination; move must leave the source
/// valid for destruction. Destroy must release exactly the target's resources.
/// Targets must remain valid when relocated as bytes and must not unwind.
pub unsafe trait Callable<A, R> {
    unsafe extern "C" fn invoke(this: *const Self, arg: A) -> R;
    unsafe extern "C" fn copy(target: *mut Self, source: *const Self);
    unsafe extern "C" fn move_(target: *mut Self, source: *mut Self);
    unsafe extern "C" fn destroy(this: *mut Self);
}

unsafe impl<A, R> Callable<A, R> for extern "C" fn(A) -> R {
    unsafe extern "C" fn invoke(this: *const Self, arg: A) -> R {
        unsafe { (*this)(arg) }
    }

    unsafe extern "C" fn copy(target: *mut Self, source: *const Self) {
        unsafe { target.write(*source) };
    }

    unsafe extern "C" fn move_(target: *mut Self, source: *mut Self) {
        unsafe { target.write(*source) };
    }

    unsafe extern "C" fn destroy(_this: *mut Self) {}
}

unsafe impl<R> Callable<ffi::c_void, R> for extern "C" fn() -> R {
    unsafe extern "C" fn invoke(this: *const Self, _arg: ffi::c_void) -> R {
        unsafe { (*this)() }
    }

    unsafe extern "C" fn copy(target: *mut Self, source: *const Self) {
        unsafe { target.write(*source) };
    }

    unsafe extern "C" fn move_(target: *mut Self, source: *mut Self) {
        unsafe { target.write(*source) };
    }

    unsafe extern "C" fn destroy(_this: *mut Self) {}
}

#[repr(C)]
pub struct CallbackHandler<C, A, R> {
    invoke: unsafe extern "C" fn(this: *const C, arg: A) -> R,
    copy: unsafe extern "C" fn(target: *mut C, source: *const C),
    move_: unsafe extern "C" fn(target: *mut C, source: *mut C),
    destroy: unsafe extern "C" fn(this: *mut C),
}

/// Erased destruction for SDK inline callback storage.
///
/// # Safety
/// Implementations must destroy the same target type stored by their handler.
#[doc(hidden)]
pub unsafe trait CallbackStorage {
    /// # Safety
    /// `storage` must contain a live target associated with this handler.
    unsafe fn destroy_storage(&self, storage: *mut ffi::c_void);
}

unsafe impl<C, A, R> CallbackStorage for CallbackHandler<C, A, R> {
    unsafe fn destroy_storage(&self, storage: *mut ffi::c_void) {
        unsafe { (self.destroy)(storage.cast()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn noop() {}

    struct Owned(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    unsafe impl Callable<u64, u64> for Owned {
        unsafe extern "C" fn invoke(_this: *const Self, arg: u64) -> u64 {
            arg
        }
        unsafe extern "C" fn copy(target: *mut Self, source: *const Self) {
            unsafe { target.write(Self((*source).0.clone())) };
        }
        unsafe extern "C" fn move_(target: *mut Self, source: *mut Self) {
            // Sharing is a valid move policy; the source remains destructible.
            unsafe { Self::copy(target, source) };
        }
        unsafe extern "C" fn destroy(this: *mut Self) {
            unsafe {
                (*this).0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ptr::drop_in_place(this);
            }
        }
    }

    #[test]
    fn owned_target_is_aligned_and_destroyed_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let drops = std::sync::Arc::new(AtomicUsize::new(0));
        let callback = Callback::new(Owned(drops.clone()));
        assert!(callback.buffer.as_ptr().cast::<Owned>().is_aligned());
        assert_eq!(
            unsafe { (callback.handler.invoke)(callback.buffer.as_ptr().cast(), 42) },
            42
        );
        drop(callback);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(std::sync::Arc::strong_count(&drops), 1);
    }

    struct Oversized([u8; 33]);
    #[repr(align(16))]
    struct Overaligned;

    macro_rules! test_callable {
        ($ty:ty) => {
            unsafe impl Callable<u64, u64> for $ty {
                unsafe extern "C" fn invoke(_this: *const Self, arg: u64) -> u64 {
                    arg
                }
                unsafe extern "C" fn copy(_target: *mut Self, _source: *const Self) {
                    unreachable!()
                }
                unsafe extern "C" fn move_(_target: *mut Self, _source: *mut Self) {
                    unreachable!()
                }
                unsafe extern "C" fn destroy(_this: *mut Self) {}
            }
        };
    }
    test_callable!(Oversized);
    test_callable!(Overaligned);

    #[test]
    #[should_panic(expected = "callback exceeds inline storage")]
    fn rejects_oversized_target() {
        let target = Oversized([0; 33]);
        assert_eq!(target.0.len(), 33);
        let _ = Callback::new(target);
    }

    #[test]
    #[should_panic(expected = "callback exceeds inline alignment")]
    fn rejects_overaligned_target() {
        let _ = Callback::new(Overaligned);
    }

    #[test]
    fn callback_matches_sdk_layout() {
        assert_eq!(mem::size_of::<VoidFunctionPointerCallback>(), 0x28);
        assert_eq!(mem::align_of::<VoidFunctionPointerCallback>(), 8);
        let callback = VoidFunctionPointerCallback::new(noop as extern "C" fn());
        assert_eq!(
            ptr::addr_of!(callback.handler) as usize - ptr::addr_of!(callback) as usize,
            0x20
        );
    }
}
