use std::ffi::CString;
use std::pin::Pin;

use cxx::UniquePtr;

use crate::ffi::{glue, RED4ext};
use crate::function::REDFunction;
use crate::interop::{fnv1a64, Ref};

pub type RegisterCallback = extern "C" fn();

pub type ExternCName = UniquePtr<RED4ext::CName>;

#[inline]
pub fn get_rtti<'a>() -> Pin<&'a mut RED4ext::IRTTISystem> {
    unsafe { Pin::new_unchecked(&mut *(RED4ext::CRTTISystem::Get() as *mut RED4ext::IRTTISystem)) }
}

#[inline]
pub fn get_cname(str: &str) -> ExternCName {
    RED4ext::CName::make_unique(fnv1a64(str))
}

pub fn get_function(fn_name: ExternCName) -> *mut RED4ext::CBaseFunction {
    get_rtti().GetFunction(fn_name) as *mut _
}

pub fn get_class(name: ExternCName) -> *mut RED4ext::CClass {
    get_rtti().GetClass(name)
}

pub fn get_type(name: ExternCName) -> *const RED4ext::CBaseRTTIType {
    get_rtti().GetType(name)
}

pub fn get_method(this: Ref<RED4ext::IScriptable>, fn_name: ExternCName) -> *mut RED4ext::CBaseFunction {
    unsafe {
        let typ = Pin::new_unchecked(this.instance.as_mut().unwrap()).GetType();
        Pin::new_unchecked(typ.as_mut().unwrap()).GetFunction(fn_name) as *mut _
    }
}

pub fn get_static_method(class: ExternCName, fn_name: ExternCName) -> *mut RED4ext::CBaseFunction {
    unsafe {
        let typ = get_class(class);
        Pin::new_unchecked(typ.as_mut().unwrap()).GetFunction(fn_name) as *mut _
    }
}

pub fn register_function(name: &str, func: REDFunction) {
    let c_str = CString::new(name).unwrap();
    unsafe {
        let func = glue::CreateNativeFunction(c_str.as_ptr(), c_str.as_ptr(), func as *mut _);
        get_rtti().RegisterFunction(func);
    }
}

#[macro_export]
macro_rules! register_function {
    ($name:literal,$fun:expr) => {{
        unsafe extern "C" fn native_impl(
            ctx: *mut $crate::RED4ext::IScriptable,
            frame: *mut $crate::RED4ext::CStackFrame,
            ret: *mut std::ffi::c_void,
            _unk: i64,
        ) {
            $crate::function::REDInvokable::invoke($fun, ctx, frame, ret);
            std::pin::Pin::new_unchecked(&mut *frame).as_mut().Step();
        }

        $crate::rtti::register_function($name, native_impl)
    }};
}
