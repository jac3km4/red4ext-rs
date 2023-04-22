use std::mem;
use std::pin::Pin;

use red4ext_sys::ffi;
use red4ext_sys::interop::{Mem, StackArg};

use crate::conv::{fill_memory, from_frame, FromRED, IntoRED, NativeRepr};
use crate::error::InvokeError;
use crate::rtti::RTTI;
use crate::types::{CName, IScriptable, Ref, VoidPtr};

pub(crate) type REDFunction =
    unsafe extern "C" fn(*mut ffi::IScriptable, *mut ffi::CStackFrame, Mem, i64);
type REDType = *const ffi::CBaseRTTIType;

pub trait Invocable<A, R>: private_invocable::Sealed<A, R> {
    const ARG_TYPES: &'static [CName];
    const RETURN_TYPE: CName;

    fn invoke(self, ctx: *mut ffi::IScriptable, frame: *mut ffi::CStackFrame, mem: Mem);
}

macro_rules! impl_invocable {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            impl<$($types,)* R, FN> Invocable<($($types,)*), R> for FN
            where
                FN: Fn($($types,)*) -> R,
                $($types: FromRED, $types::Repr: Default,)*
                R: IntoRED
            {
                const ARG_TYPES: &'static [CName] = &[$(CName::new($types::Repr::NAME),)*];
                const RETURN_TYPE: CName = CName::new(R::Repr::NAME);

                #[inline]
                fn invoke(self, ctx: *mut ffi::IScriptable, frame: *mut ffi::CStackFrame, mem: Mem) {
                    $(let $types: $types = from_frame(frame);)*
                    let res = self($($types,)*);
                    fill_memory(res, mem);
                }
            }
        )*

        mod private_invocable {
            pub trait Sealed<A, R> {}
            $(
                impl<$($types,)* R, FN> Sealed<($($types,)*), R> for FN
                    where
                        FN: Fn($($types,)*) -> R {}
            )*
        }
    };
}

impl_invocable!(
    (),
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);

#[macro_export]
macro_rules! call {
    ($fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {{
        let mut rtti = $crate::rtti::RTTI::get();
        $crate::call_direct!(
            rtti,
            $crate::types::Ref::null(),
            rtti.get_function($crate::types::CName::new($fn_name)),
            ($($args),*) -> $rett
        ).expect($fn_name)
    }};
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {{
        let mut rtti = $crate::rtti::RTTI::get();
        let this = $this;
        $crate::call_direct!(
            rtti,
            this.clone(),
            $crate::rtti::RTTI::get_method(this, $crate::types::CName::new($fn_name)),
            ($($args),*) -> $rett
        ).expect($fn_name)
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! call_direct {
    ($rtti:expr, $this:expr, $func:expr, ($( $args:expr ),*) -> $rett:ty) => {{
        let args = ($($crate::invocable::into_type_and_repr(&mut $rtti, $args)),*);
        let args = $crate::invocable::Args::to_stack_args(&args);
        let res: ::std::result::Result<$rett, _> = $crate::invocable::invoke($this, $func, &args);
        res
    }}
}

pub fn invoke<R>(
    this: Ref<IScriptable>,
    fun: *mut ffi::CBaseFunction,
    args: &[StackArg],
) -> Result<R, InvokeError>
where
    R: FromRED,
    R::Repr: Default,
{
    let mut ret = R::Repr::default();

    unsafe {
        let Some(fun_ref) = fun.as_mut() else {
            return Err(InvokeError::FunctionNotFound);
        };
        validate_invocation(args, CName::new(R::Repr::NAME), fun_ref)?;

        ffi::execute_function(
            VoidPtr(this.as_ptr() as _),
            Pin::new_unchecked(fun_ref),
            mem::transmute(&mut ret),
            &args,
        )
    };
    Ok(R::from_repr(&ret))
}

fn validate_invocation(
    args: &[StackArg],
    expected_return: CName,
    fun: &ffi::CBaseFunction,
) -> Result<(), InvokeError> {
    let params = ffi::get_parameters(fun);
    if params.len() != args.len() {
        return Err(InvokeError::InvalidArgCount {
            given: args.len(),
            expected: params.len(),
        });
    }

    for (index, (&param, arg)) in params.iter().zip(args).enumerate() {
        match unsafe { ffi::get_property_type(param).as_ref() } {
            Some(expected) if std::ptr::eq(expected, arg.inner_type()) => {}
            Some(expected) => {
                let expected = ffi::resolve_cname(&expected.get_name());
                return Err(InvokeError::ArgMismatch { expected, index });
            }
            _ => return Err(InvokeError::InvalidFunction),
        }
    }

    match unsafe {
        ffi::get_return(fun)
            .as_ref()
            .and_then(|prop| ffi::get_property_type(prop).as_ref())
    } {
        Some(actual) if actual.get_name() == expected_return => Ok(()),
        Some(actual) => {
            let expected = ffi::resolve_cname(&actual.get_name());
            Err(InvokeError::ReturnMismatch { expected })
        }
        None if expected_return == CName::new("Void") => Ok(()),
        None => Err(InvokeError::ReturnMismatch { expected: "Void" }),
    }
}

#[inline]
pub fn into_type_and_repr<A: IntoRED>(rtti: &mut RTTI<'_>, val: A) -> (REDType, A::Repr) {
    (rtti.get_type(CName::new(A::Repr::NAME)), val.into_repr())
}

#[inline]
pub fn get_invocable_types<F: Invocable<A, R>, A, R>(_f: &F) -> (&[CName], CName) {
    (F::ARG_TYPES, F::RETURN_TYPE)
}

pub trait Args: private_args::Sealed {
    type StackArgs;

    fn to_stack_args(&self) -> Self::StackArgs;
}

macro_rules! impl_args {
    ($( ($( $ids:ident ),*) ),*) => {
        $(
            #[allow(unused_parens, non_snake_case)]
            impl <$($ids),*> Args for ($((REDType, $ids)),*) {
                type StackArgs = [StackArg; count_args!($($ids)*)];

                #[inline]
                fn to_stack_args(&self) -> Self::StackArgs {
                    let ($($ids),*) = self;
                    [$(StackArg::new($ids.0, &$ids.1 as *const _ as _)),*]
                }
            }
        )*

        #[allow(unused_parens)]
        mod private_args {
            use super::*;
            pub trait Sealed {}
            $(impl <$($ids),*> Sealed for ($((REDType, $ids)),*) {})*
        }
    };
}

macro_rules! count_args {
    ($id:ident $( $t:tt )*) => {
        1 + count_args!($($t)*)
    };
    () => { 0 }
}

impl_args!(
    (),
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);
