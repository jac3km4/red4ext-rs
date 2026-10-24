#include "bridge.hpp"

#include <cstddef>
#include <memory>
#include <type_traits>

#include <RED4ext/ResourceReference.hpp>

namespace
{
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

extern "C" void red4ext_rs_destroy_resource_ref(void* resource_ref) noexcept
{
    std::destroy_at(static_cast<ErasedResourceRef*>(resource_ref));
}
