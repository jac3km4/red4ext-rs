use std::ffi::CString;
use std::pin::Pin;
use std::thread;
use std::time::Duration;

use cxx::UniquePtr;

use crate::ffi::{glue, RED4ext};
use crate::function::REDFunction;
use crate::interop::{fnv1a64, Ref};

pub type RegisterCallback = extern "C" fn();

type ExternCName = UniquePtr<RED4ext::CName>;

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

pub fn get_method(this: Ref<RED4ext::IScriptable>, fn_name: ExternCName) -> *mut RED4ext::CBaseFunction {
    unsafe {
        let typ = Pin::new_unchecked(this.instance.as_mut().unwrap()).GetType();
        Pin::new_unchecked(typ.as_mut().unwrap()).GetFunction(fn_name) as *mut _
    }
}

pub fn get_static_method(class: ExternCName, fn_name: ExternCName) -> *mut RED4ext::CBaseFunction {
    unsafe {
        let typ = get_rtti().GetClass(class);
        Pin::new_unchecked(typ.as_mut().unwrap()).GetFunction(fn_name) as *mut _
    }
}

pub fn get_type(name: ExternCName) -> *const RED4ext::CBaseRTTIType {
    get_rtti().GetType(name)
}

pub fn on_register(register: RegisterCallback, post_register: RegisterCallback) {
    thread::spawn(move || {
        thread::sleep(Duration::from_micros(1));
        unsafe { glue::AddRTTICallback(register as *mut _, post_register as *mut _, true) };
    });
}

pub fn register_function(name: &str, func: REDFunction) {
    let c_str = CString::new(name).unwrap();
    unsafe {
        let func = glue::CreateNativeFunction(c_str.as_ptr(), c_str.as_ptr(), func as *mut _);
        get_rtti().RegisterFunction(func);
    }
}
