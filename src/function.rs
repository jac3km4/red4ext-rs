use std::mem;

use erasable::ErasedPtr;

use crate::ffi::{glue, RED4ext};
use crate::interop::{FromRED, IntoRED, Mem, Ref, StackArg};
use crate::rtti;

pub type REDFunction = unsafe extern "C" fn(*mut RED4ext::IScriptable, *mut RED4ext::CStackFrame, Mem, i64);
type REDType = *const RED4ext::CBaseRTTIType;

pub trait REDInvokable<A, R> {
    fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: Mem);
}

macro_rules! impl_invokable {
    ($( $types:ident ),*) => {
        #[allow(unused_variables)]
        impl<$($types,)* R, FN> REDInvokable<($($types,)*), R> for FN
        where
            FN: Fn($($types,)*) -> R,
            $($types: FromRED,)*
            R: IntoRED
        {
            #[inline]
            fn invoke(self, ctx: *mut RED4ext::IScriptable, frame: *mut RED4ext::CStackFrame, mem: Mem) {
                $(let casey::lower!($types) = FromRED::from_red(frame);)*
                let res = self($(casey::lower!($types),)*);
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

pub fn invoke<R: FromRED, const N: usize>(
    this: Ref<RED4ext::IScriptable>,
    fun: *mut RED4ext::CBaseFunction,
    args: [(REDType, ErasedPtr); N],
) -> R {
    let arg_iter = args
        .into_iter()
        .map(|(typ, val)| StackArg::new(typ, val.as_ptr() as Mem));
    let args: [StackArg; N] = array_init::from_iter(arg_iter).unwrap();
    let mut ret = R::Repr::default();

    let vector = unsafe { glue::ConstructArgs(mem::transmute(args.as_ptr()), args.len() as u64) };
    unsafe { RED4ext::ExecuteFunction(this.instance as _, fun, mem::transmute(&mut ret), vector) };
    R::from_repr(ret)
}

#[inline]
pub fn get_argument_type<A: IntoRED>(_val: &A) -> *const RED4ext::CBaseRTTIType {
    rtti::get_type(rtti::get_cname(A::type_name()))
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
            $crate::rtti::get_function($crate::rtti::get_cname($fn_name)),
            ($($args),*) -> $rett
        )
    };
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        $crate::invoke!(
            $this.clone(),
            $crate::rtti::get_method($this, $crate::rtti::get_cname($fn_name)),
            ($($args),*) -> $rett
        )
    };
}

#[macro_export]
macro_rules! call_static {
    ($class:literal :: $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        $crate::invoke!(
            $crate::interop::Ref::null(),
            $crate::rtti::get_static_method($crate::rtti::get_cname($class), $crate::rtti::get_cname($fn_name)),
            ($($args),*) -> $rett
        )
    };
}
