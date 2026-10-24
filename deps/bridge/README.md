# Native destruction bridge

This bridge targets only the SDK revision pinned by this repository. It adds no
engine addresses and links against the SDK already built by `build.rs`.

## Resource reference erasure

`ResourceReference<T>` contains a `ResourcePath` followed by a
`SharedPtr<ResourceToken<T>>` (SDK `ResourceReference.hpp`, offsets 0 and 8).
Its defaulted destructor only destroys that shared pointer. The bridge invokes
that destructor as `ResourceReference<CResource>`.

This is an SDK ABI erasure, not a general claim that unrelated C++ template
specializations can be aliased. The pinned SDK supplies the following evidence:

- `ResourceToken<T>` in `ResourceLoader.hpp` has **no virtual destructor and is
  not polymorphic**. There is no template-independent token base to destroy.
- Its only T-dependent fields are `WeakPtr<ResourceToken<T>> self` and
  `Handle<T> resource`; neither changes layout with T. Its destructor body uses
  only template-independent state and SDK cancellation/destruction routines.
- The SDK already exposes tokens erased to `ResourceToken<CResource>` through
  `ResourceLoader::tokens`, `ResourceLoader::failed`, and token `dependencies`.
  `LoadAsync<T>` and `FindToken<T>` pass typed token storage to the same native
  functions for every T.
- `SharedPtr` destruction performs the SDK's strong/weak release and
  `Memory::Delete<ResourceToken<CResource>>`, using `EngineAllocator`. Token
  dependencies and members are then destroyed by C++, not manually recursed.
- `Handle<T>` ultimately calls `Memory::Delete`. `CResource` inherits
  `ISerializable` at offset zero; `ISerializable` has a virtual destructor and
  virtual `GetAllocator`. Thus erasing the resource handle to `CResource` still
  destroys the dynamic resource type and frees it with its own allocator.

The ABI assumption is restricted to SDK resource types with the SDK's
zero-offset CResource base, not arbitrary Rust/C++ types. Compile-time checks
cover representative derived-resource layouts and the virtual destructor.
Review this evidence when updating the SDK pin. There is no claim of portable
ISO C++ cross-specialization aliasing outside this native ABI contract.

The Rust null-token drop test calls the real SDK-backed bridge without loading
the game. Nonempty tokens require game relocations and allocators and must be
validated in-game. A naive fake `ISerializable` subclass is not a standalone
fixture: its base constructor accesses the game's global serializable counter
(`ISerializable-inl.hpp`), even when the virtual allocator is overridden.
