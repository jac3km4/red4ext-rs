use std::pin::Pin;
use std::{fmt, ptr};

use red4ext_sys::ffi;

use crate::invocable::RedFunction;
use crate::types::{CName, RefShared, VoidPtr};

pub struct Rtti<'a> {
    inner: Pin<&'a mut ffi::IRttiSystem>,
}

impl<'a> Rtti<'a> {
    #[inline]
    pub fn get() -> Self {
        Self {
            inner: unsafe { Pin::new_unchecked(&mut *ffi::get_rtti()) },
        }
    }

    #[inline]
    pub fn get_class(&mut self, name: CName) -> *const ffi::CClass {
        self.inner.as_mut().get_class(name)
    }

    #[inline]
    pub fn get_type(&mut self, name: CName) -> *mut ffi::CBaseRttiType {
        self.inner.as_mut().get_type(name)
    }

    #[inline]
    pub fn get_function(&mut self, fn_name: CName) -> *mut ffi::CBaseFunction {
        self.inner.as_mut().get_function(fn_name) as *mut _
    }

    pub fn get_method(
        this: RefShared<ffi::IScriptable>,
        fn_name: CName,
    ) -> *mut ffi::CBaseFunction {
        let typ = Self::class_of(this);
        unsafe { typ.as_ref() }
            .map(|t| ffi::get_method(t, &fn_name) as _)
            .unwrap_or(ptr::null_mut())
    }

    pub fn get_static_method(typ: *const ffi::CClass, fn_name: CName) -> *mut ffi::CBaseFunction {
        unsafe { typ.as_ref() }
            .map(|t| ffi::get_static_method(t, &fn_name) as _)
            .unwrap_or(ptr::null_mut())
    }

    pub fn register_function(
        &mut self,
        name: &str,
        func: RedFunction,
        args: &[CName],
        ret: CName,
    ) -> Result<(), FailedBindings> {
        let mut failed = vec![];
        unsafe {
            let func = ffi::new_native_function(
                name,
                name,
                VoidPtr(func as *mut _),
                args,
                ret,
                &mut failed,
            );
            self.inner.as_mut().register_function(func);
        }
        if failed.is_empty() {
            Ok(())
        } else {
            Err(FailedBindings(failed))
        }
    }

    #[inline]
    pub fn class_of(this: RefShared<ffi::IScriptable>) -> *const ffi::CClass {
        unsafe {
            this.as_ptr()
                .as_mut()
                .map(|is| Pin::new_unchecked(is).get_class())
        }
        .unwrap_or(ptr::null_mut())
    }

    #[inline]
    pub fn type_name_of(typ: *const ffi::CBaseRttiType) -> Option<CName> {
        unsafe { typ.as_ref().map(ffi::CBaseRttiType::get_name) }
    }
}

#[derive(Debug)]
pub struct FailedBindings(Vec<usize>);

impl fmt::Display for FailedBindings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to bind arguments at indexes:")?;
        self.0.iter().try_for_each(|e| write!(f, " {}", e))?;
        write!(
            f,
            ", this is only an issue if you don't define this function in redscript"
        )
    }
}

#[macro_export]
macro_rules! register_function {
    ($name:literal,$fun:expr) => {{
        unsafe extern "C" fn native_impl(
            ctx: *mut $crate::ffi::IScriptable,
            frame: *mut $crate::ffi::CStackFrame,
            ret: *mut ::std::ffi::c_void,
            _unk: i64,
        ) {
            #[cfg(debug_assertions)]
            if let Some(err) = ::std::panic::catch_unwind(|| {
                $crate::invocable::Invocable::invoke($fun, ctx, frame, ret)
            })
            .err()
            .and_then(|err| err.downcast::<::std::string::String>().ok())
            {
                $crate::error!("Function '{}' has panicked: {err}", $name);
            }

            #[cfg(not(debug_assertions))]
            $crate::invocable::Invocable::invoke($fun, ctx, frame, ret);

            ::std::pin::Pin::new_unchecked(&mut *frame).step();
        }

        let (arg_types, ret_type) = $crate::invocable::get_invocable_types(&$fun);
        if let Err(err) =
            $crate::rtti::Rtti::get().register_function($name, native_impl, arg_types, ret_type)
        {
            #[cfg(debug_assertions)]
            $crate::warn!("Registering '{}' has partially failed: {}", $name, err);
        }
    }};
}
