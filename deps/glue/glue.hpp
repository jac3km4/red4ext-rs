#pragma once

#include <RED4ext/RED4ext.hpp>

using namespace RED4ext;

namespace glue {

  CGlobalFunction* CreateNativeFunction(const char* aFullName, const char* aShortName, void* aFunc)
  {
    CBaseFunction::Flags flags = { .isNative = true, .isStatic = true };
    auto func = CGlobalFunction::Create(aFullName, aShortName, (ScriptingFunction_t<void*>)aFunc);
    func->flags = flags;
    return func;
  }

  void AddRTTICallback(void* aRegFunc, void* aPostRegFunc, bool aUnused)
  {
      RTTIRegistrator::Add((RTTIRegistrator::CallbackFunc)aRegFunc, (RTTIRegistrator::CallbackFunc)aPostRegFunc, aUnused);
  }

  void ConstructStringAt(CString* addr, const char* aText, Memory::IAllocator* aAllocator) {
      CString cstr(aText, aAllocator);
      *addr = cstr;
  }

  std::vector<CStackType> ConstructArgs(CStackType* args, uint64_t n)
  {
      std::vector<CStackType> vec(args, args + n);
      return vec;
  }

  void DefinePlugin(void* ptr, const uint16_t* name, const uint16_t* author, uint8_t major, uint16_t minor, uint32_t patch)
  {
      auto aInfo = (PluginInfo*)ptr;
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
