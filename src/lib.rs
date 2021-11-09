pub mod function;
pub mod interop;
pub mod plugin;

mod prelude {
    pub use cstr::cstr;
    pub use red4ext_rs_macros::redscript;

    pub use crate::ffi::RED4ext;
    pub use crate::function::{register_native, REDInvokable};
    pub use crate::register_function;
}

autocxx::include_cpp! {
  #include "RED4ext/RED4ext.hpp"

  safety!(unsafe)

  generate!("RED4ext::ExecuteGlobalFunction")
  generate!("RED4ext::ExecuteFunction")
  generate!("RED4ext::GetParameter")

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
