#pragma once

#include <RED4ext/RTTITypes.hpp>
#include <RED4ext/RTTISystem.hpp>
using namespace RED4ext;

namespace glue {

  CGlobalFunction* CreateNativeFunction(const char* aFullName, const char* aShortName, void* aFunc, bool native)
  {
    CBaseFunction::Flags flags = { .isNative = native, .isStatic = true };
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

}
