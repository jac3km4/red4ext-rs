use std::ffi::CStr;

use crate::repr::{FromRepr, IntoRepr, NativeRepr};
use crate::types::{
    CName, Class, Function, FunctionHandler, GlobalFunction, IScriptable, Method, PoolRef,
    StackFrame,
};
use crate::VoidPtr;

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
