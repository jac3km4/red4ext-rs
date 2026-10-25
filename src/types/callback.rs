use std::marker::PhantomData;
use std::{ffi, ptr};

/// No-argument C callback used for RTTI registration.
pub type VoidFunctionPointerCallback =
    Callback<extern "C" fn(), CallbackHandler<extern "C" fn(), ffi::c_void, ()>>;

/// Inline callable using the SDK
/// [Callback](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Callback.hpp) layout.
#[repr(C)]
pub struct Callback<F, H: 'static> {
    callback: PhantomData<F>,
    buffer: [u8; 0x40],
    handler: &'static H,
}

impl<F: Callable<A, R>, A, R> Callback<F, CallbackHandler<F, A, R>> {
    const HANDLER: CallbackHandler<F, A, R> = CallbackHandler {
        invoke: F::invoke,
        copy: F::copy,
        move_: F::move_,
        destroy: F::destroy,
    };

    pub fn new(callback: F) -> Self {
        let mut buffer = [0u8; 0x40];
        unsafe { ptr::write(buffer.as_mut_ptr().cast::<F>(), callback) };
        Self {
            callback: PhantomData,
            buffer,
            handler: &Self::HANDLER,
        }
    }
}

pub unsafe trait Callable<A, R> {
    extern "C" fn invoke(&self, arg: A) -> R;
    extern "C" fn copy(target: &mut Self, source: &Self);
    extern "C" fn move_(target: &mut Self, source: &mut Self);
    extern "C" fn destroy(&mut self);
}

unsafe impl<A, R> Callable<A, R> for extern "C" fn(A) -> R {
    extern "C" fn invoke(&self, arg: A) -> R {
        self(arg)
    }

    extern "C" fn copy(target: &mut Self, source: &Self) {
        *target = *source;
    }

    extern "C" fn move_(target: &mut Self, source: &mut Self) {
        *target = *source;
    }

    extern "C" fn destroy(&mut self) {}
}

unsafe impl<R> Callable<ffi::c_void, R> for extern "C" fn() -> R {
    extern "C" fn invoke(&self, _arg: ffi::c_void) -> R {
        self()
    }

    extern "C" fn copy(target: &mut Self, source: &Self) {
        *target = *source;
    }

    extern "C" fn move_(target: &mut Self, source: &mut Self) {
        *target = *source;
    }

    extern "C" fn destroy(&mut self) {}
}

#[repr(C)]
#[allow(dead_code)]
pub struct CallbackHandler<C, A, R> {
    invoke: extern "C" fn(this: &C, arg: A) -> R,
    copy: extern "C" fn(target: &mut C, source: &C),
    move_: extern "C" fn(target: &mut C, source: &mut C),
    destroy: extern "C" fn(this: &mut C),
}
