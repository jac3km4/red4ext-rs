use std::ffi::CStr;
use std::pin::Pin;

use casey::lower;

use crate::ffi::RED4ext;
use crate::interop::{FromRED, IntoRED};

#[macro_export]
macro_rules! register_function {
    ($id:ident) => {
        register_native(cstr::cstr!($id), $id);
    };
}

pub fn register_native(name: &CStr, func: REDFunction) {
    unsafe {
        let rtti = Pin::new_unchecked(&mut *(RED4ext::CRTTISystem::Get() as *mut RED4ext::IRTTISystem));
        let func = RED4ext::CGlobalFunction::Create(
            name.as_ptr(),
            name.as_ptr(),
            func as *mut autocxx::c_void,
            true,
        );
        rtti.RegisterFunction(func);
    }
}

pub trait IntoREDFunction<A, R> {
    fn invoke(
        self,
        ctx: *mut RED4ext::IScriptable,
        frame: *mut RED4ext::CStackFrame,
        mem: *mut autocxx::c_void,
    );
}

macro_rules! impl_function_unit {
($( $types:ident ),*) => {
    #[allow(unused_variables)]
    impl<$($types,)*> IntoREDFunction<($($types,)*), ()> for fn($($types,)*)
    where
        $($types: FromRED,)*
    {
      fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: *mut autocxx::c_void) {
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

macro_rules! impl_function_ret {
($( $types:ident ),*) => {
    #[allow(unused_variables)]
    impl<$($types,)* R> IntoREDFunction<($($types,)*), R> for fn($($types,)*) -> R
    where
        $($types: FromRED,)*
        R: IntoRED
    {
      fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: *mut autocxx::c_void) {
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

pub type REDFunction =
    unsafe extern "C" fn(*mut RED4ext::IScriptable, *mut RED4ext::CStackFrame, *mut autocxx::c_void, i64);
