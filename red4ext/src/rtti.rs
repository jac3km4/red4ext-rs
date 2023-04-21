use std::pin::Pin;

use red4ext_sys::ffi;

use crate::invokable::REDFunction;
use crate::types::{CName, Ref, VoidPtr};

pub struct RTTI<'a> {
    inner: Pin<&'a mut ffi::IRTTISystem>,
}

impl<'a> RTTI<'a> {
    #[inline]
    pub fn get() -> Self {
        Self {
            inner: unsafe { Pin::new_unchecked(&mut *ffi::get_rtti()) },
        }
    }

    #[inline]
    pub fn get_class(&mut self, name: CName) -> *const ffi::CClass {
        self.inner.as_mut().get_class(name)
    }

    #[inline]
    pub fn get_type(&mut self, name: CName) -> *const ffi::CBaseRTTIType {
        self.inner.as_mut().get_type(name)
    }

    #[inline]
    pub fn get_function(&mut self, fn_name: CName) -> *mut ffi::CBaseFunction {
        self.inner.as_mut().get_function(fn_name) as *mut _
    }

    pub fn get_method(this: Ref<ffi::IScriptable>, fn_name: CName) -> *mut ffi::CBaseFunction {
        unsafe {
            let typ = Self::class_of(this);
            (*typ).get_function(fn_name) as *mut _
        }
    }

    pub fn get_static_method(&mut self, class: CName, fn_name: CName) -> *mut ffi::CBaseFunction {
        unsafe {
            let typ = self.get_class(class);
            (*typ).get_function(fn_name) as *mut _
        }
    }

    pub fn register_function(&mut self, name: &str, func: REDFunction, args: &[CName], ret: CName) {
        unsafe {
            let func = ffi::new_native_function(name, name, VoidPtr(func as *mut _), args, ret);
            self.inner.as_mut().register_function(func);
        }
    }

    #[inline]
    pub fn class_of(this: Ref<ffi::IScriptable>) -> *const ffi::CClass {
        unsafe { Pin::new_unchecked(&mut *this.instance).get_class() }
    }

    #[inline]
    pub fn type_name_of(typ: *const ffi::CBaseRTTIType) -> CName {
        unsafe { (*typ).get_name() }
    }
}

#[macro_export]
macro_rules! register_function {
    ($name:literal,$fun:expr) => {{
        unsafe extern "C" fn native_impl(
            ctx: *mut $crate::ffi::IScriptable,
            frame: *mut $crate::ffi::CStackFrame,
            ret: *mut std::ffi::c_void,
            _unk: i64,
        ) {
            $crate::invokable::REDInvokable::invoke($fun, ctx, frame, ret);
            std::pin::Pin::new_unchecked(&mut *frame).as_mut().step();
        }

        let (arg_types, ret_type) = $crate::invokable::get_invokable_types(&$fun);
        $crate::rtti::RTTI::get().register_function($name, native_impl, arg_types, ret_type)
    }};
}
