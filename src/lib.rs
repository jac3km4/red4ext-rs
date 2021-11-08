
pub mod plugin;

autocxx::include_cpp! {
  #include "RED4ext/RED4ext.hpp"
  #include "RED4ext/Scripting/Natives/ScriptGameInstance.hpp"

  generate!("RED4ext::ExecuteGlobalFunction")
  generate!("RED4ext::ExecuteFunction")
  generate!("RED4ext::GetParameter")

  generate!("RED4ext::IScriptable")
  generate!("RED4ext::BaseStream")
  generate!("RED4ext::Memory::IAllocator")
  generate!("RED4ext::CBaseRTTIType")
  generate!("RED4ext::IRTTISystem")
  generate!("RED4ext::CRTTISystem")
  generate!("RED4ext::ScriptGameInstance")
  generate!("RED4ext::CStackFrame")
  generate!("RED4ext::Handle")
  generate!("RED4ext::CGlobalFunction")
  generate!("RED4ext::RTTIRegistrator")
  generate!("RED4ext::PluginInfo")
  generate!("RED4ext::PluginHandle")
  generate!("RED4ext::IRED4ext")
  generate!("RED4ext::CName")
  generate!("RED4ext::CNamePool")
  generate!("RED4ext::CClass")

  safety!(unsafe_ffi)
}
