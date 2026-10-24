#pragma once

namespace RED4ext
{
struct ISerializable;
struct CClass;
}

// Called with a live, nonnull instance after the caller releases the last strong
// reference and its implicit weak reference. Does not modify reference counts.
extern "C" void red4ext_rs_destroy_handle_instance(RED4ext::ISerializable* instance) noexcept;

// Permanent native class metadata; callback drops Rust fields while suppressing
// only this instance's embedded IScriptable drop (C++ destroys that base).
using Red4extRustDestroy = void (*)(void*);
extern "C" void red4ext_rs_configure_native_class(RED4ext::CClass* type, Red4extRustDestroy destroy) noexcept;

// Returns false without touching memory for unregistered types (StructExport).
// Requires a fully initialized, zero-offset IScriptable base for registered types.
extern "C" bool red4ext_rs_bind_native_instance(const RED4ext::CClass* type, void* instance) noexcept;

// The caller transfers one ResourceReference<T> owner, with the pinned SDK layout.
extern "C" void red4ext_rs_destroy_resource_ref(void* resource_ref) noexcept;
