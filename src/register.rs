use std::ffi::CStr;
use std::pin::Pin;
use std::thread;
use std::time::Duration;

use crate::ffi::RED4ext;
use crate::function::REDFunction;
use crate::interop::Mem;

pub type RegisterCallback = extern "C" fn();

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
        register_native(cstr!($fun), $fun);
    };
    ($name:expr, $fun:ident) => {
        register_native(cstr!($name), $fun);
    };
}
