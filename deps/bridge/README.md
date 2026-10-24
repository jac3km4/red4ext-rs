# Native destruction bridge

This bridge targets only the SDK revision pinned by this repository. It adds no
engine addresses and links against the SDK already built by `build.rs`.

## Final handle instance release

`red4ext_rs_destroy_handle_instance(ISerializable*)` accepts a live, nonnull
instance after the caller has released the last strong reference and its implicit
weak reference, matching `SharedPtrBase::DecRef` followed by `Handle::~Handle`.
It does not touch reference counts. It calls the virtual `CanBeDestructed` and,
if allowed, SDK `Memory::Delete`, which obtains the virtual allocator before
invoking the virtual destructor and freeing the object. A veto leaves the
instance alive, exactly as in the SDK. The generated Rust binding is
`crate::raw::root::red4ext_rs_destroy_handle_instance`.

## Rust-exported native instance dispatch

Patching the `CClass` RTTI vtable is not sufficient for handle deletion: SDK
`Memory::Delete` calls the **instance** virtual allocator and destructor. The
SDK's inherited `ISerializable::GetAllocator` returns `ScriptAllocator`, whereas
Rust `NativeClass::alloc` uses `CClass::GetAllocator` (`RTTIAllocator` in the
pinned SDK). The inherited base destructor also cannot drop Rust-owned fields.

Only `ClassExport` registers a permanent class -> Rust destructor + allocator
entry. `StructExport` never opts in. The registry is mutex-protected; lookup
copies metadata and releases the mutex before callbacks or SDK calls. Entries,
RTTI classes, and Rust callback code must remain valid for all instances' lives;
unloading a plugin while its classes/instances remain registered is unsupported.

After `T::default` initializes an exported class, binding sets `nativeType` and
copies the ABI vptr from a live, fieldless `IScriptable`-derived prototype. Size,
alignment, inheritance, and virtual signature contracts are compile-time
asserted. The compiler generates `GetNativeType`, `GetAllocator`, and virtual
(including deleting) destructor dispatch; no Rust destructor-slot ABI is guessed.
The allocator is captured from the same permanent RTTI class used by allocation.
SDK `Memory::Delete` captures that allocator before virtual destruction.

The shim destructor calls Rust `drop_in_place<T>` under a thread-local,
exact-instance-address base-drop suppression guard. All Rust fields (including
nested native owners at other addresses) are dropped normally. The embedded
`IScriptable` Rust drop alone is skipped, because C++ automatically destroys
`IScriptable`, its value holder, and `ISerializable` weak members once after the
callback. RTTI `DestructCls` has no automatic C++ shim destructor and therefore
continues to perform the full Rust drop without suppression. Rust destructors
must not unwind across the C ABI.

`clone_from` retains the existing initialized-destination contract: user code
must preserve destination base identity, weak handles, and value holder. Dispatch
is rebound after non-self assignment; this does not repair an invalid Clone
implementation or change assignment into placement construction. A standalone
`clone_native_base` keeps the SDK base vptr, not the shim (there is no enclosing
Rust payload to destroy); RTTI construction/assignment binds complete instances.

Game-free tests cover exact/scoped/thread-local suppression, Rust payload and
nested-owner destruction versus RTTI destruction, and an unregistered value-type
bind that must not touch memory. A live shim prototype cannot be created in a
standalone test: its base constructor and destructor require game relocations.
The real handle -> virtual shim -> Rust fields -> native base -> matching free
sequence still requires in-game validation; compile-time ABI contracts and DLL
linking do not replace it.

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
