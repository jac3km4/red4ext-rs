#pragma once

#include "rust/cxx.h"
#include <RED4ext/RED4ext.hpp>
#include <cstdint>

using namespace RED4ext;

namespace glue {

using VoidPtr = void*;

CGlobalFunction* CreateNativeFunction(
    rust::Str aFullName,
    rust::Str aShortName,
    const VoidPtr aFunc,
    rust::Slice<const CName> params,
    CName ret,
    rust::Vec<size_t>& failed)
{
    CBaseFunction::Flags flags = { .isNative = true, .isStatic = true };
    auto func = CGlobalFunction::Create(std::string(aFullName).c_str(),
        std::string(aShortName).c_str(),
        (ScriptingFunction_t<void*>)aFunc);
    func->flags = flags;
    size_t index = 0;

    for (auto param : params) {
        if (!func->AddParam(param, "")) {
            failed.push_back(index);
        }
        index++;
    }
    func->SetReturnType(ret);
    return func;
}

IRTTISystem* GetRTTI()
{
    return CRTTISystem::Get();
}

void AddRTTICallback(
    const VoidPtr aRegFunc,
    const VoidPtr aPostRegFunc,
    bool aUnused)
{
    IRTTISystem* rtti = GetRTTI();

    rtti->AddRegisterCallback((RTTIRegistrator::CallbackFunc)aRegFunc);
    rtti->AddPostRegisterCallback((RTTIRegistrator::CallbackFunc)aPostRegFunc);
}

void ConstructStringAt(
    CString* addr,
    rust::Str aText,
    Memory::IAllocator* aAllocator)
{
    *addr = CString(std::string(aText).c_str(), aAllocator);
}

void DestructString(CString* addr)
{
    addr->~CString();
}

bool Execute(
    ScriptInstance aInstance,
    CBaseFunction& aFunc,
    VoidPtr aOut,
    rust::Slice<const CStackType> args)
{
    std::vector<CStackType> vec(args.data(), args.data() + args.length());
    return ExecuteFunction(aInstance, &aFunc, aOut, vec);
}

void DefinePlugin(
    PluginInfo* aInfo,
    const uint16_t* name,
    const uint16_t* author,
    uint8_t major,
    uint16_t minor,
    uint32_t patch)
{
    aInfo->name = (wchar_t*)name;
    aInfo->author = (wchar_t*)author;
    aInfo->version = RED4EXT_SEMVER(major, minor, patch);
    aInfo->runtime = RED4EXT_RUNTIME_LATEST;
    aInfo->sdk = RED4EXT_SDK_LATEST;
}

uint32_t GetSdkVersion()
{
    return RED4EXT_API_VERSION_LATEST;
}

void AllocArray(VoidPtr array, uint32_t cap, uint32_t elemSize)
{
    constexpr uint32_t alignment = 8;

    using func_t = void (*)(VoidPtr aThis, uint32_t aCapacity,
        uint32_t aElementSize, uint32_t aAlignment,
        void (*a5)(int64_t, int64_t, int64_t, int64_t));
    RelocFunc<func_t> func(Addresses::DynArray_Realloc);
    func(array, cap, elemSize, alignment, nullptr);
}

void FreeArray(VoidPtr ptr, size_t elemSize)
{
    auto array = (DynArray<uint8_t>*)ptr;
    if (array->capacity) {
        auto allocatorPtr = reinterpret_cast<size_t>(&array->entries[array->capacity * elemSize]);
        auto allocator = reinterpret_cast<Memory::IAllocator*>(AlignUp(allocatorPtr, sizeof(void*)));
        allocator->Free(array->entries);
        array->capacity = 0;
    }
}

rust::Slice<const CProperty* const> GetParameters(const CBaseFunction& func)
{
    return rust::Slice<const CProperty* const>(func.params.entries,
        func.params.size);
}

const CProperty* GetReturn(const CBaseFunction& func)
{
    return func.returnType;
}

const CBaseRTTIType* GetPropertyType(const CProperty* prop)
{
    return prop->type;
}

rust::Str ResolveCName(const CName& cname)
{
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

CClassStaticFunction* GetStaticMethod(
    const CClass& cls,
    const CName& funcName)
{
    for (auto func : (&cls)->staticFuncs) {
        if (func->shortName == funcName || func->fullName == funcName) {
            return func;
        }
    }

    if ((&cls)->parent) {
        return GetStaticMethod(*(&cls)->parent, funcName);
    }
    return nullptr;
}

void IncRef(RefCnt* ref)
{
    ref->IncRef();
}

}
// namespace glue
