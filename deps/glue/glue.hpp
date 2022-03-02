#pragma once

#include <RED4ext/RED4ext.hpp>
#include "rust/cxx.h"

using namespace RED4ext;

namespace glue {

  using VoidPtr = void*;

  CGlobalFunction* CreateNativeFunction(rust::Str aFullName, rust::Str aShortName, const VoidPtr aFunc)
  {
    CBaseFunction::Flags flags = { .isNative = true, .isStatic = true };
    auto func = CGlobalFunction::Create(std::string(aFullName).c_str(), std::string(aShortName).c_str(), (ScriptingFunction_t<void*>)aFunc);
    func->flags = flags;
    return func;
  }

  IRTTISystem* GetRTTI() {
      return CRTTISystem::Get();
  }

  void AddRTTICallback(const VoidPtr aRegFunc, const VoidPtr aPostRegFunc, bool aUnused)
  {
      RTTIRegistrator::Add((RTTIRegistrator::CallbackFunc)aRegFunc, (RTTIRegistrator::CallbackFunc)aPostRegFunc, aUnused);
  }

  void ConstructStringAt(CString* addr, rust::Str aText, Memory::IAllocator* aAllocator) {
      CString cstr(std::string(aText).c_str(), aAllocator);
      *addr = cstr;
  }

  bool Execute(ScriptInstance aInstance, CBaseFunction* aFunc, VoidPtr aOut, const CStackType* args, uint64_t arg_count)
  {
      std::vector<CStackType> vec(args, args + arg_count);
      return ExecuteFunction(aInstance, aFunc, aOut, vec);
  }

  void DefinePlugin(PluginInfo* aInfo, const uint16_t* name, const uint16_t* author, uint8_t major, uint16_t minor, uint32_t patch)
  {
      aInfo->name = (wchar_t*)name;
      aInfo->author = (wchar_t*)author;
      aInfo->version = RED4EXT_SEMVER(major, minor, patch);
      aInfo->runtime = RED4EXT_RUNTIME_LATEST;
      aInfo->sdk = RED4EXT_SDK_LATEST;
  }

  uint32_t GetSdkVersion() {
      return RED4EXT_API_VERSION_LATEST;
  }
}
