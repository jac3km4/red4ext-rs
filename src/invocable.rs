use std::ffi::CStr;

use sealed::sealed;
use thiserror::Error;

use crate::repr::{FromRepr, IntoRepr, NativeRepr};
use crate::types::{
    CName, Class, Function, FunctionHandler, GlobalFunction, IScriptable, Method, PoolRef,
    StackArg, StackFrame,
};
use crate::VoidPtr;

#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("function not found")]
    FunctionNotFound,
    #[error("invalid number of arguments, expected {0}")]
    InvalidArgCount(u32),
    #[error("expected {expected} argument at index {index}")]
    ArgMismatch {
        expected: &'static str,
        index: usize,
    },
    #[error("return type mismatch, expected {expected}")]
    ReturnMismatch { expected: &'static str },
    #[error("could not resolve type {0}")]
    UnresolvedType(&'static str),
    #[error("execution failed")]
    ExecutionFailed,
    #[error("unexpected null reference as 'this'")]
    NullReference,
}

pub trait Invocable<A, R> {
    const ARG_TYPES: &'static [CName];
    const RETURN_TYPE: CName;

    fn invoke(self, ctx: Option<&IScriptable>, frame: &mut StackFrame, ret: &mut R);
}

macro_rules! impl_invocable {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            impl<$($types,)* R, FN> Invocable<($($types,)*), R::Repr> for FN
            where
                FN: Fn($($types,)*) -> R,
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const ARG_TYPES: &'static [CName] = &[$(CName::new($types::Repr::NATIVE_NAME),)*];
                const RETURN_TYPE: CName = CName::new(R::Repr::NATIVE_NAME);

                #[inline]
                fn invoke(self, ctx: Option<&IScriptable>, frame: &mut StackFrame, ret: &mut R::Repr) {
                    $(let $types = $types::from_repr(unsafe { frame.get_arg::<$types::Repr>() });)*
                    let res = self($($types,)*);
                    *ret = res.into_repr();
                }
            }
        )*
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

#[derive(Debug)]
pub struct FnMetadata {
    ptr: FunctionHandler<VoidPtr>,
    args: &'static [CName],
    ret: CName,
}

impl FnMetadata {
    #[inline]
    pub fn new<A, R, F: Invocable<A, R>>(ptr: FunctionHandler<VoidPtr>, _f: &F) -> Self {
        Self {
            ptr,
            args: F::ARG_TYPES,
            ret: F::RETURN_TYPE,
        }
    }

    #[inline]
    pub fn to_global(&self, name: &CStr) -> PoolRef<GlobalFunction> {
        let mut func = GlobalFunction::new(name, name, self.ptr);
        self.initialize(func.as_function_mut());
        func
    }

    #[inline]
    pub fn to_method(&self, name: &CStr, parent: &Class) -> PoolRef<Method> {
        let mut func = Method::new(name, name, parent, self.ptr);
        self.initialize(func.as_function_mut());
        func
    }

    #[inline]
    pub fn to_static_method(&self, name: &CStr, parent: &Class) -> PoolRef<Method> {
        let mut func = Method::new(name, name, parent, self.ptr);
        self.initialize(func.as_function_mut());
        func
    }

    fn initialize(&self, func: &mut Function) {
        for &arg in self.args {
            func.add_param(arg, c"", false, false);
        }
        func.set_return_type(self.ret);
        func.set_is_native(true);
    }
}

#[macro_export]
macro_rules! invocable {
    ($fun:expr) => {{
        extern "C" fn native_impl(
            ctx: Option<&$crate::types::IScriptable>,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { std::mem::transmute(ret) };
            $crate::invocable::Invocable::invoke($fun, ctx, frame, out);
            unsafe { frame.step() };
        }

        $crate::invocable::FnMetadata::new(native_impl, &$fun)
    }};
}

#[macro_export]
macro_rules! call {
    ($fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        (|| {
            $crate::systems::RttiSystem::get()
                .get_function($crate::types::CName::new($fn_name))
                .ok_or($crate::invocable::InvokeError::FunctionNotFound)?
                .execute::<_, $rett>(None, ($( $crate::repr::IntoRepr::into_repr($args), )*))
        })()
    };
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        (|| {
            $crate::types::IScriptable::class(::std::convert::AsRef::as_ref($this))
                .get_method($crate::types::CName::new($fn_name))
                .ok_or($crate::invocable::InvokeError::FunctionNotFound)?
                .as_function()
                .execute::<_, $rett>(
                    Some(::std::convert::AsRef::as_ref($this)),
                    ($( $crate::repr::IntoRepr::into_repr($args), )*)
                )
        })()
    };
}

#[sealed]
pub trait Args {
    type Array<'a>: AsRef<[StackArg<'a>]>
    where
        Self: 'a;

    fn to_array(&mut self) -> Result<Self::Array<'_>, InvokeError>;
}

macro_rules! impl_args {
    ($( ($( $ids:ident ),*) ),*) => {
        $(
            #[allow(unused_parens, non_snake_case)]
            #[sealed]
            impl <$($ids: NativeRepr),*> Args for ($($ids,)*) {
                type Array<'a> = [StackArg<'a>; count_args!($($ids)*)] where Self: 'a;

                #[inline]
                fn to_array(&mut self) -> Result<Self::Array<'_>, InvokeError> {
                    let ($($ids,)*) = self;
                    Ok([$(
                        StackArg::new($ids).ok_or_else(||
                            InvokeError::UnresolvedType($ids::NATIVE_NAME)
                        )?),*
                    ])
                }
            }
        )*
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
