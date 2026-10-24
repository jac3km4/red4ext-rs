#pragma once

namespace RED4ext
{
struct ISerializable;
}

// Called with a live, nonnull instance after the caller releases the last strong
// reference and its implicit weak reference. Does not modify reference counts.
extern "C" void red4ext_rs_destroy_handle_instance(RED4ext::ISerializable* instance) noexcept;

// The caller transfers one ResourceReference<T> owner, with the pinned SDK layout.
extern "C" void red4ext_rs_destroy_resource_ref(void* resource_ref) noexcept;
