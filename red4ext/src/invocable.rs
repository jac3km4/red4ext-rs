use std::mem;
use std::pin::Pin;

use red4ext_sys::ffi;
use red4ext_sys::interop::StackArg;
use red4ext_types::Mem;

use crate::conv::{fill_memory, from_frame, FromRepr, IntoRepr, NativeRepr};
use crate::error::InvokeError;
use crate::rtti::Rtti;
use crate::types::{CName, IScriptable, RefShared, VoidPtr};

pub(crate) type RedFunction =
    unsafe extern "C" fn(*mut ffi::IScriptable, *mut ffi::CStackFrame, Mem, i64);
type RedType = *const ffi::CBaseRttiType;

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
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const ARG_TYPES: &'static [CName] = &[$(get_native_cname::<$types::Repr>(),)*];
                const RETURN_TYPE: CName = get_native_cname::<R::Repr>();

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
        $crate::call!([$fn_name] ($($args),*) -> $rett)
    }};
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {{
        $crate::call!($this, [$fn_name] ($($args),*) -> $rett)
    }};
    ([$fn_name:expr] ($( $args:expr ),*)) => {
        $crate::call!([$fn_name] ($($args),*) -> ())
    };
    ([$cls_name:literal] :: [$fn_name:literal] ($( $args:expr ),*) -> $rett:ty) => {{
        let mut rtti = $crate::rtti::Rtti::get();
        match $crate::call_direct!(
            rtti,
            $crate::types::RefShared::null(),
            $crate::rtti::Rtti::get_static_method(
                rtti.get_type($crate::types::CName::new($cls_name)) as *const $crate::ffi::CClass,
                $crate::types::CName::new($fn_name)),
            ($($args),*) -> $rett
        ) {
            Ok(res) => res,
            Err(err) => $crate::invocable::raise_invoke_error($fn_name, err)
        }
    }};
    ([$fn_name:expr] ($( $args:expr ),*) -> $rett:ty) => {{
        let mut rtti = $crate::rtti::Rtti::get();
        let function = rtti.get_function($crate::types::CName::new($fn_name));
        match $crate::call_direct!(rtti, $crate::types::RefShared::null(), function, ($($args),*) -> $rett) {
            Ok(res) => res,
            Err(err) => $crate::invocable::raise_invoke_error($fn_name, err)
        }
    }};
    ($this:expr, [$fn_name:expr] ($( $args:expr ),*)) => {
        $crate::call!($this, [$fn_name] ($($args),*) -> ())
    };
    ($this:expr, [$fn_name:expr] ($( $args:expr ),*) -> $rett:ty) => {{
        #[allow(unused_mut, unused_variables)]
        let mut rtti = $crate::rtti::Rtti::get();
        let this = $crate::types::Ref::as_shared(&$this).as_scriptable();
        let method = $crate::rtti::Rtti::get_method(this.clone(), $crate::types::CName::new($fn_name));
        match $crate::call_direct!(rtti, this.clone(), method, ($($args),*) -> $rett) {
            Ok(res) => res,
            Err(err) => $crate::invocable::raise_invoke_error($fn_name, err)
        }
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

#[inline]
pub fn invoke<R>(
    this: RefShared<IScriptable>,
    fun: *mut ffi::CBaseFunction,
    args: &[StackArg],
) -> Result<R, InvokeError>
where
    R: FromRepr,
    R::Repr: Default,
{
    // don't inline to avoid exploding code size of macros
    #[inline(never)]
    fn invoke_shared(
        this: RefShared<IScriptable>,
        fun: *mut ffi::CBaseFunction,
        args: &[StackArg],
        native_return_type: CName,
        out: VoidPtr,
    ) -> Result<(), InvokeError> {
        let fun_ref = unsafe { fun.as_mut() };
        let Some(fun_ref) = fun_ref else {
            return Err(InvokeError::FunctionNotFound);
        };
        validate_invocation(args, native_return_type, fun_ref)?;
        let this = VoidPtr(this.as_ptr() as _);
        unsafe { ffi::execute_function(this, Pin::new_unchecked(fun_ref), out, args) };
        Ok(())
    }

    let mut ret = R::Repr::default();
    let ret_ptr = unsafe { mem::transmute::<&mut R::Repr, VoidPtr>(&mut ret) };
    invoke_shared(this, fun, args, CName::new(R::Repr::NATIVE_NAME), ret_ptr)?;
    Ok(R::from_repr(ret))
}

fn validate_invocation(
    args: &[StackArg],
    native_return_type: CName,
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
        Some(actual) if actual.get_name() == native_return_type => Ok(()),
        Some(actual) => {
            let expected = ffi::resolve_cname(&actual.get_name());
            Err(InvokeError::ReturnMismatch { expected })
        }
        None if native_return_type == CName::new("Void") => Ok(()),
        None => Err(InvokeError::ReturnMismatch { expected: "Void" }),
    }
}

// this implements a fallback that is necessary for scripted ref types
// because they do not exist yet when functions are registered
const fn get_native_cname<A: NativeRepr>() -> CName {
    let bytes = &A::NATIVE_NAME.as_bytes();
    if bytes.len() < 8 {
        return CName::new(A::NATIVE_NAME);
    }
    if matches!(
        &[bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],],
        b"handle:"
    ) {
        return CName::new("handle:IScriptable");
    }
    if matches!(
        &[bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]],
        b"whandle:"
    ) {
        return CName::new("whandle:IScriptable");
    }
    CName::new(A::NATIVE_NAME)
}

#[inline]
pub fn into_type_and_repr<A: IntoRepr>(rtti: &mut Rtti<'_>, val: A) -> (RedType, A::Repr) {
    (
        rtti.get_type(CName::new(A::Repr::NATIVE_NAME)),
        val.into_repr(),
    )
}

#[inline]
pub fn get_invocable_types<F: Invocable<A, R>, A, R>(_f: &F) -> (&[CName], CName) {
    (F::ARG_TYPES, F::RETURN_TYPE)
}

// don't inline to avoid exploding code size of macros
#[inline(never)]
pub fn raise_invoke_error<A>(source: &str, error: InvokeError) -> A {
    panic!("failed to invoke {source}: {error}")
}

pub trait Args: private_args::Sealed {
    type StackArgs;

    fn to_stack_args(&self) -> Self::StackArgs;
}

macro_rules! impl_args {
    ($( ($( $ids:ident ),*) ),*) => {
        $(
            #[allow(unused_parens, non_snake_case)]
            impl <$($ids),*> Args for ($((RedType, $ids)),*) {
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
            $(impl <$($ids),*> Sealed for ($((RedType, $ids)),*) {})*
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
