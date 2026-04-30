# red4ext-rs Agent Guidelines

You are an expert Rust developer specializing in the `red4ext-rs` project, a Rust wrapper around the `RED4ext.SDK` for Cyberpunk 2077 modding.

## Commands

Executable commands to validate and build the project:

```bash
# Validate syntax and types (Expect C++ bindgen failures on Linux)
cargo check

# Compile to a Windows DLL (`cdylib`)
cargo build --target x86_64-pc-windows-msvc

# Lint the Rust code
cargo clippy --all-targets --all-features -- -D warnings
```

## Boundaries

### Always do
- Implement manual memory deallocation via custom `Drop` implementations for Rust collections wrapping RED4ext C++ types (e.g., `RedHashMap` wrapping `red::HashMap`) using the engine's allocator to prevent memory leaks.
- Ensure exposed Rust function signatures consist only of supported types (e.g., do not use `i128`).
- Use the `Plugin` trait and `export_plugin_symbols!(YourPlugin)` macro to export plugin symbols correctly.
- Ensure native class structs include a `base` field (e.g., `base: IScriptable`) and implement `ScriptClass` with `type Kind = class_kind::Native`.

### Ask first
- Before modifying `build.rs` or `bindgen` configurations which integrate with the underlying `RED4ext.SDK`.

### Never do
- Never attempt to run or "fix" `cargo test` failures locally on a Linux host. The C++ `RED4ext.SDK` strictly requires Windows headers (e.g., `<Windows.h>`) and macro/offset assertions fail during cross-compilation. Always rely on the CI pipeline.

## Project Structure

Map of the critical source files and their purposes:

- `src/lib.rs`: Defines the `Plugin` and `PluginOps` traits, plugin lifecycle logic, `export_plugin_symbols!` macro, and logging utilities.
- `src/invocable.rs`: Houses the macros (`call!`, `global!`, `method!`) used to invoke REDscript or native functions and export Rust functions.
- `src/export.rs`: Contains the `exports!`, `methods!`, and `static_methods!` macros, as well as `ClassExport` and `StructExport` builders.
- `src/class.rs`: Defines the `ScriptClass` trait to categorize structs via `class_kind::Native` or `class_kind::Scripted`.
- `deps/RED4ext.SDK/`: The submodule containing the upstream C++ SDK.

## Code Style

### String Handling
Use the `U16CStr` type from `widestring` (via the `wcstr!` macro) when wide C strings are expected by the engine.

```rust
// Preferred: Use wcstr! for constant U16CStr values
const AUTHOR: &'static U16CStr = wcstr!("me");
```

### Exporting Classes
```rust
// Preferred: Use the ClassExport builder pattern
exports![ClassExport::<MyClass>::builder()
    .base("IScriptable")
    .methods(methods![
        c"GetValue" => MyClass::value,
    ])
    .build()]
```

### Function Invocation
```rust
// Preferred: Use the call! macro with exact type signatures
let size = call!(player, "GetDeviceActionMaxQueueSize;" () -> i32).unwrap();
```

## Testing

Framework: Built-in `cargo test`.
Environment: Tests *must* be run in a Windows environment or via the CI pipeline due to `<Windows.h>` dependencies in the C++ SDK.

## Git Workflow

Branch naming: Use descriptive names (e.g., `feat/add-new-export-macro`).
Commit format: Follow conventional commits (e.g., `feat: enhance ClassExport builder`).
PR conventions: Never push directly to the main branch. Ensure all CI checks pass since local Linux testing is not viable.
