use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::MaybeUninit;

use sealed::sealed;
use thiserror::Error;

use crate::class::ClassKind;
use crate::repr::{FromRepr, IntoRepr, NativeRepr};
use crate::types::{
    CName, Class, Function, FunctionFlags, FunctionHandler, GlobalFunction, IScriptable, Method,
    PoolRef, Ref, ScriptRef, StackArg, StackFrame, StaticMethod,
};
use crate::{ScriptClass, VoidPtr};

/// A reflected call could not be resolved, type-checked, or executed.
#[derive(Debug, Error)]
pub enum InvokeError {
    /// No global or static function has the requested full RTTI name.
    #[error("function could not be found by full name '{0}'")]
    FunctionNotFound(&'static str),
    /// The requested class could not be resolved.
    #[error("class could not be found by name '{0}'")]
    ClassNotFound(&'static str),
    /// No instance method matches the name; includes candidate method names.
    #[error(
        "method could not be found by full name '{0}', available options: {options}",
        options = .1.iter()
            .fold(String::new() ,|mut acc, el| {
                if !acc.is_empty() {
                    acc.push_str(", ");
                }
                acc.push('\'');
                acc.push_str(el.as_str());
                acc.push('\'');
                acc
            })
    )]
    MethodNotFound(&'static str, Box<[CName]>),
    /// The argument count differs from the reflected signature.
    #[error("invalid number of arguments, expected {expected} for {function}")]
    InvalidArgCount {
        /// The function's RTTI name.
        function: &'static str,
        /// The number of parameters in the reflected signature.
        expected: u32,
    },
    /// An argument's engine type differs from the reflected parameter type.
    #[error("expected '{expected}' argument type at index {index} for '{function}'")]
    ArgMismatch {
        /// The function's RTTI name.
        function: &'static str,
        /// The parameter's engine RTTI type name.
        expected: &'static str,
        /// The zero-based argument index.
        index: usize,
    },
    /// The requested return type differs from the reflected return type.
    #[error("return type mismatch, expected '{expected}' for '{function}'")]
    ReturnMismatch {
        /// The function's RTTI name.
        function: &'static str,
        /// The reflected return type's RTTI name, or `Void`.
        expected: &'static str,
    },
    /// An argument or return type is not registered in engine RTTI.
    #[error("could not resolve type {0}")]
    UnresolvedType(&'static str),
    /// The engine reported that execution failed.
    #[error("execution of '{0}' has failed")]
    ExecutionFailed(&'static str),
    /// The instance receiver is a null reference to the named class.
    #[error("the 'this' pointer for class '{0}' was null")]
    NullReceiver(&'static str),
}

impl InvokeError {
    #[doc(hidden)]
    #[cold]
    pub fn new_method_not_found<'a>(
        name: &'static str,
        options: impl IntoIterator<Item = &'a Method>,
    ) -> InvokeError {
        let options = options
            .into_iter()
            .map(|m| m.as_function().name())
            .collect();
        InvokeError::MethodNotFound(name, options)
    }
}

/// Rust functions exportable as global functions or static methods.
/// Arguments use [`FromRepr`]; return values use [`IntoRepr`].
/// Normally used through [`global!`](crate::global), not called directly.
#[sealed]
pub trait GlobalInvocable<A, R> {
    /// The argument and return types exposed to engine RTTI.
    const FN_TYPE: FunctionType;

    /// Reads arguments from a matching engine stack frame and calls the function.
    /// Writes the result when return storage is provided; `ctx` may be null.
    fn invoke(
        self,
        ctx: *mut IScriptable,
        frame: &mut StackFrame,
        ret: Option<&mut MaybeUninit<R>>,
    );
}

macro_rules! impl_global_invocable {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            #[sealed]
            impl<$($types,)* R, Func> GlobalInvocable<fn($($types,)*), R::Repr> for Func
            where
                Func: Fn($($types,)*) -> R,
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const FN_TYPE: FunctionType = FunctionType {
                    args: &[$(CName::new($types::Repr::NAME),)*],
                    ret: CName::new(R::Repr::NAME)
                };

                #[inline]
                fn invoke(self, _ctx: *mut IScriptable, frame: &mut StackFrame, ret: Option<&mut MaybeUninit<R::Repr>>) {
                    $(let $types = unsafe { frame.get_arg::<$types>() };)*
                    let res = self($($types,)*);
                    if let Some(ret) = ret {
                        unsafe { ret.as_mut_ptr().write(res.into_repr()) };
                    }
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
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G)
);

macro_rules! impl_global_invocable_with_ref {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            #[sealed]
            impl<This, $($types,)* R, Func> GlobalInvocable<for<'a> fn(ScriptRef<'a, This>, $($types,)*), R::Repr> for Func
            where
                This: NativeRepr,
                Func: for<'a> Fn(ScriptRef<'a, This>, $($types,)*) -> R,
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const FN_TYPE: FunctionType = FunctionType {
                    args: &[CName::new(<ScriptRef<'_, This>>::NAME), $(CName::new($types::Repr::NAME),)*],
                    ret: CName::new(R::Repr::NAME)
                };

                #[inline]
                fn invoke(self, _ctx: *mut IScriptable, frame: &mut StackFrame, ret: Option<&mut MaybeUninit<R::Repr>>) {
                    let this = unsafe { frame.get_arg() };
                    $(let $types = unsafe { frame.get_arg::<$types>() };)*
                    let res = self(this, $($types,)*);
                    if let Some(ret) = ret {
                        unsafe { ret.as_mut_ptr().write(res.into_repr()) };
                    }
                }
            }
        )*
    };
}

impl_global_invocable_with_ref!(
    (),
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F)
);

/// Rust functions exportable as native instance methods with a shared receiver.
/// Arguments use [`FromRepr`]; return values use [`IntoRepr`].
/// Normally used through [`method!`](crate::method), not called directly.
#[sealed]
pub trait MethodInvocable<Ctx, A, R> {
    /// The argument and return types exposed to engine RTTI, excluding the receiver.
    const FN_TYPE: FunctionType;

    /// Reads arguments from a matching engine stack frame and calls the method on `ctx`.
    /// Writes the result when return storage is provided.
    fn invoke(self, ctx: &Ctx, frame: &mut StackFrame, ret: Option<&mut MaybeUninit<R>>);
}

macro_rules! impl_method_invocable {
    ($( ($( $types:ident ),*) ),*) => {
        $(
            #[allow(non_snake_case, unused_variables)]
            #[sealed]
            impl<Ctx, $($types,)* R, FN> MethodInvocable<Ctx, fn($($types,)*), R::Repr> for FN
            where
                FN: Fn(&Ctx, $($types,)*) -> R,
                $($types: FromRepr, $types::Repr: Default,)*
                R: IntoRepr
            {
                const FN_TYPE: FunctionType = FunctionType {
                    args: &[$(CName::new($types::Repr::NAME),)*],
                    ret: CName::new(R::Repr::NAME)
                };

                #[inline]
                fn invoke(self, ctx: &Ctx, frame: &mut StackFrame, ret: Option<&mut MaybeUninit<R::Repr>>) {
                    $(let $types = unsafe { frame.get_arg::<$types>() };)*
                    let res = self(ctx, $($types,)*);
                    if let Some(ret) = ret {
                        unsafe { ret.as_mut_ptr().write(res.into_repr()) };
                    }
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
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G)
);

/// Engine RTTI argument and return types inferred from an exportable Rust function.
/// Describes the signature registered with engine functions.
///
/// SDK: [Functions](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Functions.hpp).
#[derive(Debug)]
pub struct FunctionType {
    args: &'static [CName],
    ret: CName,
}

impl FunctionType {
    fn initialize_func(&self, func: &mut Function) {
        for &arg in self.args {
            func.add_param(arg, c"", false, false);
        }
        func.set_return_type(self.ret);
    }
}

/// A global function or static method definition, created with [`global!`](crate::global).
#[derive(Debug)]
pub struct GlobalMetadata {
    name: &'static CStr,
    func: FunctionHandler<IScriptable, VoidPtr>,
    typ: FunctionType,
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

    /// Creates a global RTTI function without registering it.
    /// Requires initialized engine RTTI and registered argument and return types.
    pub fn to_rtti(&self) -> PoolRef<GlobalFunction> {
        let mut flags = FunctionFlags::default();
        flags.set_is_native(true);
        flags.set_is_final(true);
        flags.set_is_static(true);
        let mut func = GlobalFunction::new(self.name, self.name, self.func, flags);
        self.typ.initialize_func(func.as_function_mut());
        func
    }

    /// Creates a static RTTI method for `class` without adding it to the class.
    /// Requires initialized engine RTTI and registered argument and return types.
    pub fn to_rtti_static_method(&self, class: &Class) -> PoolRef<StaticMethod> {
        let mut flags = FunctionFlags::default();
        flags.set_is_native(true);
        flags.set_is_final(true);
        flags.set_is_static(true);

        let mut func = StaticMethod::new(self.name, self.name, class, self.func, flags);
        self.typ.initialize_func(func.as_function_mut());
        func
    }
}

/// A native instance-method definition, created with [`method!`](crate::method).
#[derive(Debug)]
pub struct MethodMetadata<Ctx> {
    name: &'static CStr,
    func: FunctionHandler<Ctx, VoidPtr>,
    typ: FunctionType,
    parent: PhantomData<fn() -> *const Ctx>,
    is_event: bool,
    is_final: bool,
}

// Instance handlers receive the native object, never scripted field storage.
impl<Ctx: ScriptClass<Kind = crate::class_kind::Native>> MethodMetadata<Ctx> {
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

    /// Creates an RTTI method for `class` without adding it to the class.
    /// `class` must describe `Ctx`. Requires initialized engine RTTI and registered
    /// argument and return types.
    pub fn to_rtti(&self, class: &Class) -> PoolRef<Method> {
        let mut flags = FunctionFlags::default();
        flags.set_is_native(true);
        flags.set_is_event(self.is_event);
        flags.set_is_final(self.is_final);

        let mut func = Method::new(self.name, self.name, class, self.func, flags);
        self.typ.initialize_func(func.as_function_mut());
        func
    }
}

/// Creates metadata for a global function or static method.
/// The name is a C string literal such as `c"Adder"`; register globals with
/// [`GlobalExport`](crate::GlobalExport) in [`exports!`](crate::exports).
///
/// # Example
/// ```rust
/// use red4ext_rs::{GlobalInvocable, GlobalMetadata, global};
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
        unsafe extern "C" fn native_impl(
            ctx: *mut $crate::types::IScriptable,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { (ret as *mut ::std::mem::MaybeUninit<_>).as_mut() };
            $crate::GlobalInvocable::invoke($fun, ctx, frame, out);
            unsafe { frame.step() };
        }

        $crate::GlobalMetadata::new($name, native_impl, &$fun)
    }};
}

/// Creates metadata for an instance method on a native [`ScriptClass`].
/// The method takes `&self`; use interior mutability for field changes.
/// Prefix the name with `event` or `final` to set the corresponding method flag.
/// Scripted field layouts cannot be used as native instance headers.
///
/// ```compile_fail
/// use red4ext_rs::{ScriptClass, class_kind, method};
/// struct Fields;
/// unsafe impl ScriptClass for Fields {
///     type Kind = class_kind::Scripted;
///     const NAME: &'static str = "Fields";
/// }
/// impl Fields { fn value(&self) -> i32 { 1 } }
/// let _ = method!(c"Value", Fields::value);
/// ```
/// Usually used in conjunction with the
/// [`methods!`](crate::methods) macro.
#[macro_export]
macro_rules! method {
    ($name:literal, $ty:ident::$id:ident $($mods:ident)*) => {{
        unsafe extern "C" fn native_impl(
            ctx: *mut $ty,
            frame: &mut $crate::types::StackFrame,
            ret: $crate::VoidPtr,
            _unk: i64,
        ) {
            let out = unsafe { (ret as *mut ::std::mem::MaybeUninit<_>).as_mut() };
            if !ctx.is_aligned() { return; }
            let Some(ctx) = (unsafe { ctx.as_ref() }) else { return };
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

/// Calls a reflected function using stack arguments.
/// Use the full RTTI name, including mangled parameter types where required.
/// Instance calls take a receiver first. Returns `Result<ReturnType, InvokeError>`.
/// Arguments use [`IntoRepr`] and the return value uses [`FromRepr`].
/// Requires initialized engine RTTI.
///
/// SDK: [Stack arguments](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Stack.hpp).
///
/// # Example
/// ```rust
/// use red4ext_rs::{call, types::{IScriptable, Ref, CName}};
///
/// fn method_example(inst: Ref<IScriptable>) -> CName {
///    call!(inst, "GetClassName" () -> CName).unwrap()
/// }
///
/// fn global_example() -> i32 {
///    call!("OperatorAdd;Int32Int32;Int32" (1i32, 2i32) -> i32).unwrap()
/// }
///
/// fn static_example() -> f32 {
///    call!("PlayerPuppet"::"GetCriticalHealthThreshold;" () -> f32).unwrap()
/// }
/// ```
#[macro_export]
macro_rules! call {
    ($cls_name:literal :: $fn_name:literal ($( $args:expr ),*) -> $rett:ty) => {
        (|| {
            let rtti = $crate::RttiSystem::get();
            let ctx = rtti
                .resolve_static_context($crate::types::CName::new($cls_name))
                .ok_or($crate::InvokeError::ClassNotFound($cls_name))?;
            rtti
                .resolve_static_method_by_full_name(::std::concat!($cls_name, "::", $fn_name))
                .ok_or($crate::InvokeError::FunctionNotFound($fn_name))?
                .execute::<_, $rett>(
                    unsafe { ctx.instance() }.map(::std::convert::AsRef::as_ref),
                    ($( $crate::IntoRepr::into_repr($args), )*)
                )
        })()
    };
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
            let receiver = $crate::AsReceiver::as_receiver(&$this)?;
            $crate::types::IScriptable::class(receiver)
                .get_method($crate::types::CName::new($fn_name))
                .map_err(|err| $crate::InvokeError::new_method_not_found($fn_name, err))?
                .as_function()
                .execute::<_, $rett>(
                    Some(receiver),
                    ($( $crate::IntoRepr::into_repr($args), )*)
                )
        })()
    };
}

/// A receiver accepted by the instance-call form of [`call!`].
/// Supports values implementing `AsRef<IScriptable>` and class references;
/// null class references produce [`InvokeError::NullReceiver`].
#[sealed]
pub trait AsReceiver {
    #[doc(hidden)]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError>;
}

#[sealed]
impl<T: AsRef<IScriptable>> AsReceiver for T {
    #[inline]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError> {
        Ok(self.as_ref())
    }
}

#[sealed]
impl<T: ScriptClass> AsReceiver for Ref<T>
where
    <T::Kind as ClassKind<T>>::NativeType: AsRef<IScriptable>,
{
    #[inline]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError> {
        unsafe { self.instance() }
            .map(AsRef::as_ref)
            .ok_or(InvokeError::NullReceiver(T::NAME))
    }
}

#[sealed]
impl<T: ScriptClass> AsReceiver for &Ref<T>
where
    <T::Kind as ClassKind<T>>::NativeType: AsRef<IScriptable>,
{
    #[inline]
    fn as_receiver(&self) -> Result<&IScriptable, InvokeError> {
        <Ref<T> as AsReceiver>::as_receiver(*self)
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
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G)
);

#[cfg(test)]
mod handler_tests {
    use super::*;

    fn answer() -> i32 {
        42
    }

    #[test]
    fn global_handler_accepts_null_context() {
        let metadata = crate::global!(c"Answer", answer);
        // CStackFrame is plain pointer/integer/bool storage. The no-argument
        // handler only advances code and never reads the null metadata fields.
        let mut raw: crate::raw::root::RED4ext::CStackFrame = unsafe { std::mem::zeroed() };
        let mut code = [0i8; 2];
        raw.code = code.as_mut_ptr();
        let frame = unsafe { &mut *std::ptr::from_mut(&mut raw).cast::<StackFrame>() };
        let mut result = MaybeUninit::<i32>::uninit();
        unsafe { (metadata.func)(std::ptr::null_mut(), frame, result.as_mut_ptr().cast(), 0) };
        assert_eq!(unsafe { result.assume_init() }, 42);
        assert_eq!(raw.code, unsafe { code.as_mut_ptr().add(1) });
    }
}
