use std::mem;

use erasable::ErasedPtr;
use red4ext_sys::ffi;
use red4ext_sys::interop::{Mem, StackArg};

use crate::conv::{FromRED, IntoRED};
use crate::rtti;
use crate::types::{CName, Ref, VoidPtr};

pub type REDFunction = unsafe extern "C" fn(*mut ffi::IScriptable, *mut ffi::CStackFrame, Mem, i64);
type REDType = *const ffi::CBaseRTTIType;

pub trait REDInvokable<A, R> {
    const ARG_TYPES: &'static [CName];
    const RETURN_TYPE: CName;

    fn invoke(self, ctx: *mut ffi::IScriptable, frame: *mut ffi::CStackFrame, mem: Mem);
}

macro_rules! impl_invokable {
    ($( $types:ident ),*) => {
        #[allow(non_snake_case, unused_variables)]
        impl<$($types,)* R, FN> REDInvokable<($($types,)*), R> for FN
        where
            FN: Fn($($types,)*) -> R,
            $($types: FromRED,)*
            R: IntoRED
        {
            const ARG_TYPES: &'static [CName] = &[$(CName::new($types::NAME),)*];
            const RETURN_TYPE: CName = CName::new(R::NAME);

            #[inline]
            fn invoke(self, ctx: *mut ffi::IScriptable, frame: *mut ffi::CStackFrame, mem: Mem) {
                $(let $types = FromRED::from_red(frame);)*
                let res = self($($types,)*);
                IntoRED::into_red(res, mem);
            }
        }
    };
}

impl_invokable!();
impl_invokable!(A);
impl_invokable!(A, B);
impl_invokable!(A, B, C);
impl_invokable!(A, B, C, D);
impl_invokable!(A, B, C, D, E);
impl_invokable!(A, B, C, D, E, F);

#[inline]
pub fn invoke<R: FromRED, const N: usize>(
    this: Ref<ffi::IScriptable>,
    fun: *mut ffi::CBaseFunction,
    args: [(REDType, ErasedPtr); N],
) -> R {
    let arg_iter = args
        .into_iter()
        .map(|(typ, val)| StackArg::new(typ, val.as_ptr() as Mem));
    let args: [StackArg; N] = array_init::from_iter(arg_iter).unwrap();
    let mut ret = R::Repr::default();

    unsafe {
        ffi::execute_function(
            VoidPtr(this.instance as _),
            fun,
            mem::transmute(&mut ret),
            &args,
        )
    };
    R::from_repr(&ret)
}

#[inline]
pub fn get_argument_type<A: IntoRED>(_val: &A) -> *const ffi::CBaseRTTIType {
    rtti::get_type(CName::new(A::NAME))
}

#[macro_export]
macro_rules! invoke {
    ($this:expr, $func:expr, ($( $args:expr ),*) -> $rett:ty) => {
        {
            let args = [
                $(
                    ($crate::function::get_argument_type(&$args),
                     $crate::erasable::ErasablePtr::erase(std::boxed::Box::new($crate::interop::IntoRED::into_repr($args))))
                 ),*
            ];
            let res: $rett = $crate::function::invoke($this, $func, args);
            res
        }
    };
}

#[macro_export]
macro_rules! call {
    ($fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        $crate::invoke!(
            $crate::interop::Ref::null(),
            $crate::rtti::get_function($crate::interop::CName::new($fn_name)),
            ($($args),*) -> $rett
        )
    };
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        $crate::invoke!(
            $this,
            $crate::rtti::get_method($this, $crate::interop::CName::new($fn_name)),
            ($($args),*) -> $rett
        )
    };
}
