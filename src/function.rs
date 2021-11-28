use std::mem;

use erasable::ErasedPtr;
use red4ext_rs_macros::lower;

use crate::ffi::RED4ext;
use crate::interop::{FromRED, IntoRED, Mem, Ref, StackArg};
use crate::rtti;

pub type REDFunction = unsafe extern "C" fn(*mut RED4ext::IScriptable, *mut RED4ext::CStackFrame, Mem, i64);

pub trait REDInvokable<A, R> {
    fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: Mem);
}

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
impl_function_ret!(A, B, C, D, E, F);

pub fn exec_function<R: FromRED, const N: usize>(
    this: Ref<RED4ext::IScriptable>,
    fun: *mut RED4ext::CBaseFunction,
    vals: [ErasedPtr; N],
    types: [*const RED4ext::CBaseRTTIType; N],
) -> R {
    let arg_iter = vals
        .into_iter()
        .zip(types)
        .map(|(val, typ)| StackArg::new(typ, val.as_ptr() as Mem));
    let args: [StackArg; N] = array_init::from_iter(arg_iter).unwrap();
    let mut ret = R::Repr::default();

    let vector = unsafe { RED4ext::ConstructArgs(mem::transmute(args.as_ptr()), args.len() as u64) };
    unsafe { RED4ext::ExecuteFunction(this.instance as Mem, fun, mem::transmute(&mut ret), vector) };
    R::from_repr(ret)
}

#[inline]
pub fn get_argument_type<A: IntoRED>(_val: &A) -> *const RED4ext::CBaseRTTIType {
    rtti::get_type(rtti::get_cname(A::type_name()))
}

#[macro_export]
macro_rules! exec_function {
    ($this:expr, $func:expr, ($( $args:expr ),*) -> $rett:ty) => {
        {
            let types = [$(get_argument_type(&$args)),*];
            let args = [$(erasable::ErasablePtr::erase(std::boxed::Box::new(interop::IntoRED::into_repr($args)))),*];
            let res: $rett = exec_function($this, $func, args, types);
            res
        }
    };
}

#[macro_export]
macro_rules! call {
    ($fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        exec_function!(interop::Ref::null(), rtti::get_function(rtti::get_cname($fn_name)), ($($args),*) -> $rett)
    };
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        exec_function!($this.clone(), rtti::get_method($this, rtti::get_cname($fn_name)), ($($args),*) -> $rett)
    };
}

#[macro_export]
macro_rules! call_static {
    ($class:literal :: $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        exec_function!(interop::Ref::null(), rtti::get_static_method(rtti::get_cname($class), rtti::get_cname($fn_name)), ($($args),*) -> $rett)
    };
}
