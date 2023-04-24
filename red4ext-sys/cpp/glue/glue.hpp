#pragma once

#include <RED4ext/RED4ext.hpp>
#include "rust/cxx.h"

using namespace RED4ext;

namespace glue {

  using VoidPtr = void*;

  CGlobalFunction* CreateNativeFunction(rust::Str aFullName, rust::Str aShortName, const VoidPtr aFunc, rust::Slice<const CName> params, CName ret)
  {
    CBaseFunction::Flags flags = { .isNative = true, .isStatic = true };
    auto func = CGlobalFunction::Create(std::string(aFullName).c_str(), std::string(aShortName).c_str(), (ScriptingFunction_t<void*>)aFunc);
    func->flags = flags;
    for (auto param: params) {
      func->AddParam(param, "");
    }
    func->SetReturnType(ret);
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
    *addr = CString(std::string(aText).c_str(), aAllocator);
  }

  bool Execute(ScriptInstance aInstance, CBaseFunction& aFunc, VoidPtr aOut, rust::Slice<const CStackType> args)
  {
    std::vector<CStackType> vec(args.data(), args.data() + args.length());
    return ExecuteFunction(aInstance, &aFunc, aOut, vec);
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

  void AllocArray(VoidPtr array, uint32_t cap, uint32_t elemSize)
  {
    constexpr uint32_t alignment = 8;
    
    using func_t = void (*)(VoidPtr aThis, uint32_t aCapacity, uint32_t aElementSize, uint32_t aAlignment, void (*a5)(int64_t, int64_t, int64_t, int64_t));
    RelocFunc<func_t> func(Addresses::DynArray_Realloc);
    func(array, cap, elemSize, alignment, nullptr);
  }

  rust::Slice<const CProperty* const> GetParameters(const CBaseFunction& func) {
    return rust::Slice<const CProperty* const>(func.params.entries, func.params.size);
  }

  const CProperty* GetReturn(const CBaseFunction& func) {
    return func.returnType;
  }

  const CBaseRTTIType* GetPropertyType(const CProperty* prop) {
    return prop->type;
  }

  rust::Str ResolveCName(const CName& cname) {
    return rust::Str(CNamePool::Get(cname));
  }

  CClassFunction* GetMethod(const CClass& cls, const CName& fullName)
  {
    auto res = cls.funcsByName.Get(fullName);
    if (res) {
      return *res;
    }
    if (cls.parent) {
      return GetMethod(*cls.parent, fullName);
    }
    return nullptr;
  }
}
