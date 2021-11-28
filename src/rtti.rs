use std::ffi::CStr;
use std::pin::Pin;
use std::thread;
use std::time::Duration;

use crate::ffi::RED4ext;
use crate::function::REDFunction;
use crate::interop::{fnv1a64, CName, Mem, Ref};

pub type RegisterCallback = extern "C" fn();

#[inline]
pub fn get_rtti<'a>() -> Pin<&'a mut RED4ext::IRTTISystem> {
    unsafe { Pin::new_unchecked(&mut *(RED4ext::CRTTISystem::Get() as *mut RED4ext::IRTTISystem)) }
}

#[inline]
pub fn get_cname(str: &str) -> CName {
    RED4ext::CName::make_unique(fnv1a64(str))
}

pub fn get_function(fn_name: CName) -> *mut RED4ext::CBaseFunction {
    get_rtti().GetFunction(fn_name) as *mut _
}

pub fn get_method(this: Ref<RED4ext::IScriptable>, fn_name: CName) -> *mut RED4ext::CBaseFunction {
    unsafe {
        let typ = Pin::new_unchecked(this.instance.as_mut().unwrap()).GetType();
        Pin::new_unchecked(typ.as_mut().unwrap()).GetFunction(fn_name) as *mut _
    }
}

pub fn get_static_method(class: CName, fn_name: CName) -> *mut RED4ext::CBaseFunction {
    unsafe {
        let typ = get_rtti().GetClass(class);
        Pin::new_unchecked(typ.as_mut().unwrap()).GetFunction(fn_name) as *mut _
    }
}

pub fn get_type(name: CName) -> *const RED4ext::CBaseRTTIType {
    get_rtti().GetType(name)
}

pub fn on_register(register: RegisterCallback, post_register: RegisterCallback) {
    thread::spawn(move || {
        thread::sleep(Duration::from_micros(1));
        unsafe { RED4ext::RTTIRegistrator::AddHack(register as Mem, post_register as Mem, true) };
    });
}

pub fn register_native(name: &CStr, func: REDFunction) {
    unsafe {
        let rtti = Pin::new_unchecked(&mut *(RED4ext::CRTTISystem::Get() as *mut RED4ext::IRTTISystem));
        let func = RED4ext::CGlobalFunction::Create(name.as_ptr(), name.as_ptr(), func as Mem, true);
        rtti.RegisterFunction(func);
    }
}

#[macro_export]
macro_rules! register_function {
    ($fun:ident) => {
        rtti::register_native(cstr!($fun), $fun);
    };
    ($name:expr, $fun:ident) => {
        rtti::register_native(cstr!($name), $fun);
    };
}
