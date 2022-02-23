#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::missing_safety_doc)]

pub mod function;
pub mod interop;
pub mod plugin;
pub mod rtti;
pub use {casey, erasable, wchar};

pub use crate::ffi::{glue as RED4extGlue, RED4ext};

pub mod prelude {
    pub use crate::ffi::RED4ext;
    pub use crate::interop::{CName, Ref};
    pub use crate::plugin::{Plugin, Version};
    pub use crate::{call, call_static, define_plugin, define_trait_plugin, register_function};
}

autocxx::include_cpp! {
  #include "RED4ext/RED4ext.hpp"
  #include "glue.hpp"

  safety!(unsafe)

  generate!("RED4ext::ExecuteGlobalFunction")
  generate!("RED4ext::ExecuteFunction")
  generate!("RED4ext::GetParameter")

  generate!("RED4ext::IScriptable")
  generate!("RED4ext::IRTTISystem")
  generate!("RED4ext::CRTTISystem")
  generate!("RED4ext::CStackFrame")
  generate!("RED4ext::PluginInfo")
  generate!("RED4ext::CName")
  generate!("RED4ext::CClass")
  generate!("RED4ext::EMainReason")
  generate!("RED4ext::Sdk")

  generate!("glue::CreateNativeFunction")
  generate!("glue::AddRTTICallback")
  generate!("glue::ConstructStringAt")
  generate!("glue::ConstructArgs")
  generate!("glue::ScriptableTypeName")
  generate!("glue::DefinePlugin")
  generate!("glue::GetSdkVersion")
}
