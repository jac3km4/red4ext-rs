#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::missing_safety_doc)]

pub mod function;
pub mod interop;
pub mod rtti;

pub mod prelude {
    pub use erasable;
    pub use interop::Ref;
    pub use red4ext_rs_macros::redscript_export;

    pub use crate::ffi::RED4ext;
    pub use crate::function::{exec_function, get_argument_type, REDInvokable};
    pub use crate::{call, call_static, exec_function, interop, rtti};
}

autocxx::include_cpp! {
  #include "RED4ext/RED4ext.hpp"

  safety!(unsafe)

  generate!("RED4ext::ExecuteGlobalFunction")
  generate!("RED4ext::ExecuteFunction")
  generate!("RED4ext::GetParameter")
  generate!("RED4ext::ConstructArgs")

  generate!("RED4ext::IScriptable")
  generate!("RED4ext::IRTTISystem")
  generate!("RED4ext::CRTTISystem")
  generate!("RED4ext::CStackFrame")
  generate!("RED4ext::CGlobalFunction")
  generate!("RED4ext::RTTIRegistrator")
  generate!("RED4ext::PluginInfo")
  generate!("RED4ext::PluginHandle")
  generate!("RED4ext::CName")
  generate!("RED4ext::CNamePool")
  generate!("RED4ext::CClass")
  generate!("RED4ext::CString")
}
