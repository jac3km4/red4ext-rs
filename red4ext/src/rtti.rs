use std::pin::Pin;

use red4ext_sys::ffi;

use crate::invokable::{REDFunction, REDInvokable};
use crate::types::{CName, Ref, VoidPtr};

pub type RegisterCallback = extern "C" fn();

#[inline]
pub fn get_rtti<'a>() -> Pin<&'a mut ffi::IRTTISystem> {
    unsafe { Pin::new_unchecked(&mut *ffi::get_rtti()) }
}

#[inline]
pub fn get_class(name: CName) -> *const ffi::CClass {
    get_rtti().get_class(name)
}

#[inline]
pub fn get_type(name: CName) -> *const ffi::CBaseRTTIType {
    get_rtti().get_type(name)
}

#[inline]
pub fn class_of(this: Ref<ffi::IScriptable>) -> *const ffi::CClass {
    unsafe { Pin::new_unchecked(&mut *this.instance).get_class() }
}

#[inline]
pub fn get_type_name(typ: *const ffi::CBaseRTTIType) -> CName {
    unsafe { (*typ).get_name() }
}

pub fn get_function(fn_name: CName) -> *mut ffi::CBaseFunction {
    get_rtti().get_function(fn_name) as *mut _
}

pub fn get_method(this: Ref<ffi::IScriptable>, fn_name: CName) -> *mut ffi::CBaseFunction {
    unsafe {
        let typ = class_of(this);
        (*typ).get_function(fn_name) as *mut _
    }
}

pub fn get_static_method(class: CName, fn_name: CName) -> *mut ffi::CBaseFunction {
    unsafe {
        let typ = get_class(class);
        (*typ).get_function(fn_name) as *mut _
    }
}

pub fn register_function(name: &str, func: REDFunction, args: &[CName], ret: CName) {
    unsafe {
        let func = ffi::new_native_function(name, name, VoidPtr(func as *mut _), args, ret);
        get_rtti().register_function(func);
    }
}

#[inline]
pub fn get_invokable_types<F: REDInvokable<A, R>, A, R>(_f: &F) -> (&[CName], CName) {
    (F::ARG_TYPES, F::RETURN_TYPE)
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

        let (arg_types, ret_type) = $crate::rtti::get_invokable_types(&$fun);
        $crate::rtti::register_function($name, native_impl, arg_types, ret_type)
    }};
}
