use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

use sealed::sealed;
use thiserror::Error;

use crate::repr::{FromRepr, IntoRepr, NativeRepr};
use crate::types::{
    CName, ClassKind, Function, FunctionFlags, FunctionHandler, GlobalFunction, IScriptable,
    Method, PoolRef, Ref, ScriptClass, StackArg, StackFrame,
};
use crate::VoidPtr;

/// An error returned when invoking a function fails.
#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("function '{0}' not found")]
    FunctionNotFound(&'static str),
    #[error("invalid number of arguments, expected {expected} for {function}")]
    InvalidArgCount {
        function: &'static str,
        expected: u32,
    },
    #[error("expected '{expected}' argument type at index {index} for '{function}'")]
    ArgMismatch {
        function: &'static str,
        expected: &'static str,
        index: usize,
    },
    #[error("return type mismatch, expected '{expected}' for '{function}'")]
    ReturnMismatch {
        function: &'static str,
        expected: &'static str,
    },
    #[error("could not resolve type {0}")]
    UnresolvedType(&'static str),
    #[error("execution of '{0}' has failed")]
    ExecutionFailed(&'static str),
    #[error("the 'this' pointer for class '{0}' was null")]
    NullReceiver(&'static str),
}

/// A trait for functions that can be exported as global functions.
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
                    args: &[$(CName::new($types::Repr::NAME),)*],
                    ret: CName::new(R::Repr::NAME)
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

/// A trait for functions that can be exported as class methods.
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
                    args: &[$(CName::new($types::Repr::NAME),)*],
                    ret: CName::new(R::Repr::NAME)
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

/// A representation of a function type, including its arguments and return type.
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
    }
}

/// A representation of a global function, including its name, a function handler, and its type.
#[derive(Debug)]
pub struct GlobalMetadata {
    name: &'static CStr,
    func: FunctionHandler<IScriptable, VoidPtr>,
    typ: FnType,
}

impl GlobalMetadata {
    #[doc(hidden)]
    #[inline]
    pub const fn new<F: GlobalInvocable<A, R>, A, R>(
        name: &'static CStr,
        func: FunctionHandler<IScriptable, VoidPtr>,
        _f: &F,
    ) -> Self {
        Self {
            name,
            func,
            typ: F::FN_TYPE,
        }
    }

    /// Converts this metadata into a [`GlobalFunction`] instance, which can be registered with
    /// the [RttiSystem](crate::RttiSystem).
    pub fn to_rtti(&self) -> PoolRef<GlobalFunction> {
        let mut flags = FunctionFlags::default();
        flags.set_is_native(true);
        flags.set_is_final(true);
        flags.set_is_static(true);
        let mut func = GlobalFunction::new(self.name, self.name, self.func, flags);
        self.typ.initialize_func(func.as_function_mut());
        func
    }
}

/// A representation of a class method, including its name, a function handler, and its type.
#[derive(Debug)]
pub struct MethodMetadata<Ctx> {
    name: &'static CStr,
    func: FunctionHandler<Ctx, VoidPtr>,
    typ: FnType,
    parent: PhantomData<fn() -> *const Ctx>,
    is_event: bool,
    is_final: bool,
}

impl<Ctx: ScriptClass> MethodMetadata<Ctx> {
    #[doc(hidden)]
    #[inline]
    pub const fn new<F: MethodInvocable<Ctx, A, R>, A, R>(
        name: &'static CStr,
        ptr: FunctionHandler<Ctx, VoidPtr>,
        _f: &F,
    ) -> Self {
        Self {
            name,
            func: ptr,
            typ: F::FN_TYPE,
            parent: PhantomData,
            is_event: false,
            is_final: false,
        }
    }

    /// Configures this method as an event handler (called `cb` in REDscript).
    pub const fn with_is_event(mut self) -> Self {
        self.is_event = true;
        self
    }

    /// Configures this method as final (cannot be overridden).
    pub const fn with_is_final(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Converts this metadata into a [`Method`] instance, which can be registered with
    /// the [RttiSystem](crate::RttiSystem).
    pub fn to_rtti(&self) -> PoolRef<Method> {
        let mut flags = FunctionFlags::default();
        flags.set_is_native(true);
        flags.set_is_event(self.is_event);
        flags.set_is_final(self.is_final);

        let mut func = Method::new(self.name, self.name, self.func, flags);
        self.typ.initialize_func(func.as_function_mut());
        func
    }
}

/// A macro for defining global functions. Usually used in conjunction with the
/// [`exports!`](crate::exports) macro.
///
/// # Example
/// ```rust
/// use red4rs::{global, GlobalInvocable, GlobalMetadata};
///
/// fn my_global() -> GlobalMetadata {
///     global!(c"Adder", adder)
/// }
///
/// fn adder(a: i32, b: i32) -> i32 {
///     a + b
/// }
/// ```
#[macro_export]
macro_rules! global {
    ($name:literal, $fun:expr) => {{
        extern "C" fn native_impl(
            ctx: &$crate::types::IScriptable,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { std::mem::transmute(ret) };
            $crate::GlobalInvocable::invoke($fun, ctx, frame, out);
            unsafe { frame.step() };
        }

        $crate::GlobalMetadata::new($name, native_impl, &$fun)
    }};
}

/// A macro for defining class methods. Usually used in conjunction with the
/// [`methods!`](crate::methods) macro.
#[macro_export]
macro_rules! method {
    ($name:literal, $ty:ident::$id:ident $($mods:ident)*) => {{
        extern "C" fn native_impl(
            ctx: &$ty,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { ::std::mem::transmute(ret) };
            $crate::MethodInvocable::invoke($ty::$id, ctx, frame, out);
            unsafe { frame.step() };
        }

        $crate::MethodMetadata::new($name, native_impl, &$ty::$id)
            $(.$mods())?
    }};
    (event $name:literal, $ty:ident::$id:ident $($mods:ident)*) => {
        $crate::method!($name, $ty::$id with_is_event $($mods)*)
    };
    (final $name:literal, $ty:ident::$id:ident $($mods:ident)*) => {
        $crate::method!($name, $ty::$id with_is_final $($mods)*)
    }
}

/// A macro for conveniently calling functions and methods.
/// If you're calling a method, the first argument should be the instance of the class.
/// The next argument should be a full function name, which might have to include mangled names of
/// the parameter types.
///
/// # Example
/// ```rust
/// use red4rs::{call, types::{IScriptable, Ref, CName}};
///
/// fn method_example(inst: Ref<IScriptable>) -> CName {
///    call!(inst, "GetClassName" () -> CName).unwrap()
/// }
///
/// fn global_example() -> i32 {
///   call!("OperatorAdd;Int32Int32;Int32" (1i32, 2i32) -> i32).unwrap()
/// }
/// ```
#[macro_export]
macro_rules! call {
    ($fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        (|| {
            $crate::RttiSystem::get()
                .get_function($crate::types::CName::new($fn_name))
                .ok_or($crate::InvokeError::FunctionNotFound($fn_name))?
                .execute::<_, $rett>(None, ($( $crate::IntoRepr::into_repr($args), )*))
        })()
    };
    ($this:expr, $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        (|| {
            let receiver = $crate::Receiver::as_receiver(&$this)?;
            $crate::types::IScriptable::class(receiver)
                .get_method($crate::types::CName::new($fn_name))
                .ok_or($crate::InvokeError::FunctionNotFound($fn_name))?
                .as_function()
                .execute::<_, $rett>(
                    Some(receiver),
                    ($( $crate::IntoRepr::into_repr($args), )*)
                )
        })()
    };
}

/// A trait for types that can be used as the receiver of a method call.
#[sealed]
pub trait Receiver {
    #[doc(hidden)]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError>;
}

#[sealed]
impl<T: AsRef<IScriptable>> Receiver for T {
    #[inline]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError> {
        Ok(self.as_ref())
    }
}

#[sealed]
impl<T: ScriptClass> Receiver for Ref<T>
where
    <T::Kind as ClassKind<T>>::NativeType: AsRef<IScriptable>,
{
    #[inline]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError> {
        unsafe { self.instance() }
            .map(AsRef::as_ref)
            .ok_or(InvokeError::NullReceiver(T::CLASS_NAME))
    }
}

#[sealed]
#[doc(hidden)]
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
                            InvokeError::UnresolvedType($ids::NAME)
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
