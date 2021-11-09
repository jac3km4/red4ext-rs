use std::ffi::CStr;
use std::pin::Pin;

use red4ext_rs_macros::lower;

use crate::ffi::RED4ext;
use crate::interop::{FromRED, IntoRED, Mem};

pub type REDFunction = unsafe extern "C" fn(*mut RED4ext::IScriptable, *mut RED4ext::CStackFrame, Mem, i64);

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

pub trait REDInvokable<A, R> {
    fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: Mem);
}

macro_rules! impl_function_unit {
    ($( $types:ident ),*) => {
        #[allow(unused_variables)]
        impl<$($types,)*> REDInvokable<($($types,)*), ()> for fn($($types,)*)
        where
            $($types: FromRED,)*
        {
            fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: Mem) {
                $(let lower!($types) = FromRED::from_red(frame);)*
                self($(lower!($types),)*);
            }
        }
    };
}

impl_function_unit!();
impl_function_unit!(A);
impl_function_unit!(A, B);
impl_function_unit!(A, B, C);
impl_function_unit!(A, B, C, D);
impl_function_unit!(A, B, C, D, E);

macro_rules! impl_function_ret {
    ($( $types:ident ),*) => {
        #[allow(unused_variables)]
        impl<$($types,)* R> REDInvokable<($($types,)*), R> for fn($($types,)*) -> R
        where
            $($types: FromRED,)*
            R: IntoRED
        {
            fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: Mem) {
                $(let lower!($types) = FromRED::from_red(frame);)*
                let res = self($(lower!($types),)*);
                IntoRED::into_red(res, mem);
            }
        }
    };
}

impl_function_ret!();
impl_function_ret!(A);
impl_function_ret!(A, B);
impl_function_ret!(A, B, C);
impl_function_ret!(A, B, C, D);
impl_function_ret!(A, B, C, D, E);
