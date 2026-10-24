#include "bridge.hpp"

#include <cstddef>
#include <memory>
#include <cstring>
#include <exception>
#include <mutex>
#include <type_traits>
#include <unordered_map>

#include <RED4ext/ResourceReference.hpp>
#include <RED4ext/Scripting/IScriptable.hpp>
#include <RED4ext/RTTITypes.hpp>

namespace
{
struct NativeDispatch
{
    Red4extRustDestroy destroy;
    RED4ext::Memory::IAllocator* allocator;
};

struct NativeRegistry
{
    std::mutex mutex;
    std::unordered_map<const RED4ext::CClass*, NativeDispatch> classes;
};

NativeRegistry& Registry()
{
    // RTTI classes and their callbacks must outlive every instance, including
    // static teardown. Never hold this mutex while entering Rust or the SDK.
    static auto* registry = new NativeRegistry;
    return *registry;
}

bool FindDispatch(const RED4ext::CClass* type, NativeDispatch& dispatch)
{
    auto& registry = Registry();
    std::lock_guard lock(registry.mutex);
    const auto it = registry.classes.find(type);
    if (it == registry.classes.end())
        return false;
    dispatch = it->second;
    return true;
}

NativeDispatch RequireDispatch(const RED4ext::CClass* type)
{
    NativeDispatch dispatch{};
    if (!FindDispatch(type, dispatch))
        std::terminate();
    return dispatch;
}

// No fields: the existing IScriptable storage is the complete native base.
// The compiler owns all virtual destructor/deleting-destructor ABI details.
struct NativeInstance final : RED4ext::IScriptable
{
    RED4ext::CClass* GetNativeType() override { return nativeType; }
    RED4ext::Memory::IAllocator* GetAllocator() override
    {
        return RequireDispatch(nativeType).allocator;
    }
    ~NativeInstance() override
    {
        // A local prototype has no nativeType and owns no Rust payload.
        if (nativeType)
            RequireDispatch(nativeType).destroy(this);
        // Automatic IScriptable/ISerializable destruction follows exactly once.
    }
};
static_assert(sizeof(NativeInstance) == sizeof(RED4ext::IScriptable));
static_assert(alignof(NativeInstance) == alignof(RED4ext::IScriptable));
static_assert(std::has_virtual_destructor_v<NativeInstance>);
static_assert(std::is_final_v<NativeInstance>);
static_assert(std::is_convertible_v<NativeInstance*, RED4ext::IScriptable*>);
static_assert(std::is_same_v<decltype(&NativeInstance::GetNativeType), RED4ext::CClass* (NativeInstance::*)()>);
static_assert(std::is_same_v<decltype(&NativeInstance::GetAllocator), RED4ext::Memory::IAllocator* (NativeInstance::*)()>);

using ErasedResourceRef = RED4ext::ResourceReference<RED4ext::CResource>;
using ErasedToken = RED4ext::ResourceToken<RED4ext::CResource>;

// ResourceToken<T> is NOT polymorphic. Its only T-dependent members are self
// (WeakPtr) and resource (Handle). The SDK already erases token types in
// ResourceLoader::tokens and ResourceToken::dependencies. Handle destruction
// uses ISerializable's virtual destructor and dynamic allocator. See README.md.
struct DerivedResource : RED4ext::CResource
{
};
using DerivedToken = RED4ext::ResourceToken<DerivedResource>;
static_assert(!std::is_polymorphic_v<ErasedToken>);
static_assert(std::has_virtual_destructor_v<RED4ext::ISerializable>);
static_assert(sizeof(ErasedResourceRef) == 0x18);
static_assert(alignof(ErasedResourceRef) == alignof(RED4ext::ResourceReference<DerivedResource>));
static_assert(sizeof(ErasedResourceRef) == sizeof(RED4ext::ResourceReference<DerivedResource>));
static_assert(offsetof(ErasedResourceRef, token) == 0x8);
static_assert(sizeof(ErasedToken) == sizeof(DerivedToken));
static_assert(alignof(ErasedToken) == alignof(DerivedToken));
static_assert(offsetof(ErasedToken, resource) == offsetof(DerivedToken, resource));
static_assert(offsetof(ErasedToken, self) == offsetof(DerivedToken, self));
}

extern "C" void red4ext_rs_configure_native_class(RED4ext::CClass* type, Red4extRustDestroy destroy) noexcept
{
    NativeDispatch dispatch{destroy, type->GetAllocator()};
    auto& registry = Registry();
    std::lock_guard lock(registry.mutex);
    const auto [it, inserted] = registry.classes.emplace(type, dispatch);
    if (!inserted && (it->second.destroy != destroy || it->second.allocator != dispatch.allocator))
        std::terminate();
}

extern "C" bool red4ext_rs_bind_native_instance(const RED4ext::CClass* type, void* instance) noexcept
{
    NativeDispatch dispatch{};
    if (!FindDispatch(type, dispatch))
        return false;

    // A real, live prototype supplies the compiler's ABI vptr. Constructing it
    // needs initialized engine relocations, just like T::default's native base.
    NativeInstance prototype;
    void* vptr;
    std::memcpy(&vptr, &prototype, sizeof(vptr));
    auto* base = static_cast<RED4ext::IScriptable*>(instance);
    base->nativeType = const_cast<RED4ext::CClass*>(type);
    std::memcpy(instance, &vptr, sizeof(vptr));
    return true;
}

extern "C" void red4ext_rs_destroy_handle_instance(RED4ext::ISerializable* instance) noexcept
{
    if (instance->CanBeDestructed())
    {
        RED4ext::Memory::Delete(instance);
    }
}

extern "C" void red4ext_rs_destroy_resource_ref(void* resource_ref) noexcept
{
    std::destroy_at(static_cast<ErasedResourceRef*>(resource_ref));
}
