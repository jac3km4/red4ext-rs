#pragma once

// The caller transfers one ResourceReference<T> owner, with the pinned SDK layout.
extern "C" void red4ext_rs_destroy_resource_ref(void* resource_ref) noexcept;
