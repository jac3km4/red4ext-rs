use std::pin::Pin;

use crate::function::REDFunction;
use crate::interop::Ref;
use crate::prelude::CName;
use crate::{ffi, VoidPtr};

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
pub fn get_scriptable_type(this: Ref<ffi::IScriptable>) -> *const ffi::CClass {
    unsafe { Pin::new_unchecked(&mut *this.instance).get_type() }
}

#[inline]
pub fn get_type_name(class: *const ffi::CClass) -> CName {
    unsafe { Pin::new_unchecked(&*class).get_name() }
}

pub fn get_function(fn_name: CName) -> *mut ffi::CBaseFunction {
    get_rtti().get_function(fn_name) as *mut _
}

pub fn get_method(this: Ref<ffi::IScriptable>, fn_name: CName) -> *mut ffi::CBaseFunction {
    unsafe {
        let typ = get_scriptable_type(this);
        Pin::new_unchecked(&*typ).get_function(fn_name) as *mut _
    }
}

pub fn get_static_method(class: CName, fn_name: CName) -> *mut ffi::CBaseFunction {
    unsafe {
        let typ = get_class(class);
        Pin::new_unchecked(&*typ).get_function(fn_name) as *mut _
    }
}

pub fn register_function(name: &str, func: REDFunction) {
    unsafe {
        let func = ffi::new_native_function(name, name, VoidPtr(func as *mut _));
        get_rtti().register_function(func);
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
            $crate::function::REDInvokable::invoke($fun, ctx, frame, ret);
            std::pin::Pin::new_unchecked(&mut *frame).as_mut().step();
        }

        $crate::rtti::register_function($name, native_impl)
    }};
}
