use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

use sealed::sealed;
use thiserror::Error;

use crate::repr::{FromRepr, IntoRepr, NativeRepr};
use crate::types::{
    CName, Function, FunctionHandler, GlobalFunction, IScriptable, Method, PoolRef, ScriptClass,
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

#[sealed]
pub trait GlobalInvocable<A, R> {
    const FN_TYPE: FnType;

    fn invoke(self, ctx: &IScriptable, frame: &mut StackFrame, ret: &mut MaybeUninit<R>);
}

macro_rules! impl_global_invocable {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            #[sealed]
            impl<$($types,)* R, FN> GlobalInvocable<($($types,)*), R::Repr> for FN
            where
                FN: Fn($($types,)*) -> R,
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const FN_TYPE: FnType = FnType {
                    args: &[$(CName::new($types::Repr::NATIVE_NAME),)*],
                    ret: CName::new(R::Repr::NATIVE_NAME)
                };

                #[inline]
                fn invoke(self, _ctx: &IScriptable, frame: &mut StackFrame, ret: &mut MaybeUninit<R::Repr>) {
                    $(let $types = $types::from_repr(unsafe { frame.get_arg::<$types::Repr>() });)*
                    let res = self($($types,)*);
                    unsafe { ret.as_mut_ptr().write(res.into_repr()) }
                }
            }
        )*
    };
}

impl_global_invocable!(
    (),
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);

#[sealed]
pub trait MethodInvocable<Ctx, A, R> {
    const FN_TYPE: FnType;

    fn invoke(self, ctx: &Ctx, frame: &mut StackFrame, ret: &mut MaybeUninit<R>);
}

macro_rules! impl_method_invocable {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            #[sealed]
            impl<Ctx, $($types,)* R, FN> MethodInvocable<Ctx, ($($types,)*), R::Repr> for FN
            where
                FN: Fn(&Ctx, $($types,)*) -> R,
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const FN_TYPE: FnType = FnType {
                    args: &[$(CName::new($types::Repr::NATIVE_NAME),)*],
                    ret: CName::new(R::Repr::NATIVE_NAME)
                };

                #[inline]
                fn invoke(self, ctx: &Ctx, frame: &mut StackFrame, ret: &mut MaybeUninit<R::Repr>) {
                    $(let $types = $types::from_repr(unsafe { frame.get_arg::<$types::Repr>() });)*
                    let res = self(ctx, $($types,)*);
                    unsafe { ret.as_mut_ptr().write(res.into_repr()) }
                }
            }
        )*
    };
}

impl_method_invocable!(
    (),
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);

#[derive(Debug)]
pub struct FnType {
    args: &'static [CName],
    ret: CName,
}

impl FnType {
    fn initialize_func(&self, func: &mut Function) {
        for &arg in self.args {
            func.add_param(arg, c"", false, false);
        }
        func.set_return_type(self.ret);
        func.set_is_native(true);
    }
}

#[derive(Debug)]
pub struct GlobalMetadata {
    ptr: FunctionHandler<IScriptable, VoidPtr>,
    typ: FnType,
}

impl GlobalMetadata {
    #[inline]
    pub fn new<F: GlobalInvocable<A, R>, A, R>(
        ptr: FunctionHandler<IScriptable, VoidPtr>,
        _f: &F,
    ) -> Self {
        Self {
            ptr,
            typ: F::FN_TYPE,
        }
    }

    #[inline]
    pub fn to_rtti(&self, name: &CStr) -> PoolRef<GlobalFunction> {
        let mut func = GlobalFunction::new(name, name, self.ptr);
        self.typ.initialize_func(func.as_function_mut());
        func
    }
}

#[derive(Debug)]
pub struct MethodMetadata<Ctx> {
    ptr: FunctionHandler<Ctx, VoidPtr>,
    typ: FnType,
    parent: PhantomData<fn() -> *const Ctx>,
}

impl<Ctx: ScriptClass> MethodMetadata<Ctx> {
    #[inline]
    pub fn new<F: MethodInvocable<Ctx, A, R>, A, R>(
        ptr: FunctionHandler<Ctx, VoidPtr>,
        _f: &F,
    ) -> Self {
        Self {
            ptr,
            typ: F::FN_TYPE,
            parent: PhantomData,
        }
    }

    #[inline]
    pub fn to_rtti(&self, name: &CStr) -> PoolRef<Method> {
        let mut func = Method::new(name, name, self.ptr);
        self.typ.initialize_func(func.as_function_mut());
        func
    }
}

#[macro_export]
macro_rules! global {
    ($fun:expr) => {{
        extern "C" fn native_impl(
            ctx: &$crate::types::IScriptable,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { std::mem::transmute(ret) };
            $crate::invocable::GlobalInvocable::invoke($fun, ctx, frame, out);
            unsafe { frame.step() };
        }

        $crate::invocable::GlobalMetadata::new(native_impl, &$fun)
    }};
}

#[macro_export]
macro_rules! method {
    ($ty:ident::$name:ident) => {{
        extern "C" fn native_impl(
            ctx: &$ty,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { std::mem::transmute(ret) };
            $crate::invocable::MethodInvocable::invoke($ty::$name, ctx, frame, out);
            unsafe { frame.step() };
        }

        $crate::invocable::MethodMetadata::new(native_impl, &$ty::$name)
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
            $crate::types::IScriptable::class(::std::convert::AsRef::<IScriptable>::as_ref($this))
                .get_method($crate::types::CName::new($fn_name))
                .ok_or($crate::invocable::InvokeError::FunctionNotFound)?
                .as_function()
                .execute::<_, $rett>(
                    Some(::std::convert::AsRef::<IScriptable>::as_ref($this)),
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
