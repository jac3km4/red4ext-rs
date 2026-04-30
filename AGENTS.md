## Commands
```bash
# Compile to a Windows DLL (`cdylib`)
cargo build --target x86_64-pc-windows-msvc

# Lint the Rust code
cargo clippy --all-targets --all-features -- -D warnings
```

## Boundaries

### Always do
- Implement manual memory deallocation via custom `Drop` implementations for Rust collections wrapping RED4ext C++ types (e.g., `RedHashMap` wrapping `red::HashMap`) using the engine's allocator to prevent memory leaks.

### Ask first
- Modifying `Cargo.toml` dependencies.

### Never do
- Never attempt to run or "fix" `cargo test` failures locally on a Linux host. The C++ `RED4ext.SDK` strictly requires Windows headers (e.g., `<Windows.h>`) and macro/offset assertions fail during cross-compilation. Always rely on the CI pipeline.

## Project Structure
- `src/lib.rs`: Defines the `Plugin` and `PluginOps` traits, plugin lifecycle logic, `export_plugin_symbols!` macro, and logging utilities.
- `src/invocable.rs`: Houses the macros (`call!`, `global!`, `method!`) used to invoke REDscript or native functions and export Rust functions.
- `src/export.rs`: Contains the `exports!`, `methods!`, and `static_methods!` macros, as well as `ClassExport` and `StructExport` builders.
- `src/class.rs`: Defines the `ScriptClass` trait to categorize structs via `class_kind::Native` or `class_kind::Scripted`.
- `deps/RED4ext.SDK/`: The submodule containing the upstream C++ SDK.

## Testing
- **Framework:** `cargo test`

## Git Workflow
Branch naming:
  feat/[short-description]
  fix/[short-description]
  chore/[short-description]

Commit format: [prefix]: [what changed in imperative mood]
  Example: feat: add DWARF v5 support for symbols
