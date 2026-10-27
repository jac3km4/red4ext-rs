# red4ext-rs

Rust wrapper around [RED4ext.SDK](https://github.com/WopsS/RED4ext.SDK) for
building Cyberpunk 2077 plugins.

[API documentation](https://jac3km4.github.io/red4ext-rs/red4ext_rs/index.html)
· [Generated game bindings](https://github.com/jac3km4/red4ext-rs-bindings)

## Requirements

Build on **Windows x64** with:

- Rust **1.95+**, targeting `x86_64-pc-windows-msvc`.
- Visual Studio C++ build tools (x64 MSVC, C++20) and the Windows SDK.
- CMake **3.21+** on `PATH` ([SDK build configuration](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/CMakeLists.txt)).
- LLVM/libclang (`LIBCLANG_PATH` can point to the directory containing `libclang.dll`).

Use an x64 developer shell. Linux supports formatting only; rely on Windows CI
for native checks rather than attempting cross-compilation.

## Quickstart

Plugins must build as `cdylib` and call `export_plugin_symbols!` to export the
loader entry points ([SDK examples](https://github.com/WopsS/RED4ext.SDK/tree/2216befc23924b46996a9335043aebb31ba6a757/examples)).

`Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
red4ext-rs = { git = "https://github.com/jac3km4/red4ext-rs", rev = "v0.11.2" }
```

This example pins the `v0.11.2` release; development API changes may differ from that revision.

`src/lib.rs`:

```rust
use red4ext_rs::{
    Exportable, GlobalExport, Plugin, SemVer, U16CStr, export_plugin_symbols,
    exports, global, wcstr,
};

pub struct Example;

impl Plugin for Example {
    const AUTHOR: &'static U16CStr = wcstr!("me");
    const NAME: &'static U16CStr = wcstr!("example");
    const VERSION: SemVer = SemVer::new(0, 1, 0);

    fn exports() -> impl Exportable {
        exports![GlobalExport(global!(c"Add2", add2))]
    }
}

export_plugin_symbols!(Example);

fn add2(value: i32) -> i32 {
    value + 2
}
# fn main() {}
```

Build with `cargo build --release --target x86_64-pc-windows-msvc`. Copy
`target/x86_64-pc-windows-msvc/release/my_plugin.dll` into
`<game>/red4ext/plugins/my-plugin/` with [RED4ext](https://github.com/WopsS/RED4ext)
installed. Here, `<game>` is the Cyberpunk 2077 installation directory.

With [REDscript](https://github.com/jac3km4/redscript) installed, declare the function in
`<game>/r6/scripts/my-plugin.reds` (create the file and directories as needed):

```swift
native func Add2(value: Int32) -> Int32;
```

## Contributing

After cloning:

```sh
git submodule update --init --recursive
```

### Validation

```sh
# One-time setup
rustup toolchain install stable --profile minimal --component clippy
rustup toolchain install nightly-2026-10-15 --profile minimal --component rustfmt
# Every change
cargo +nightly-2026-10-15 fmt --all -- --check
# Before pushing
cargo +stable clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo +stable test --locked -p red4ext-rs --all-features
cargo +stable build --locked -p red4ext-smoke-plugin --release --target x86_64-pc-windows-msvc
```

Also lint and test without `--all-features`.
[CI](.github/workflows/ci.yml) includes Rust 1.95 checks. The smoke fixture in
`tests/fixtures/plugin` checks DLL linking, not in-game behavior.

### Native safety

Verify layouts against [SDK types](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/RTTITypes.hpp)
and ownership against the relevant SDK implementation. Release engine memory through
the matching [allocator](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Memory/Allocators.hpp),
never Rust's allocator. Do not move owners out of borrowed engine views.

Engine pools, relocations, RTTI, and singletons require initialized game systems;
plugin load does not imply that every system is ready. Reference counts do not
prevent native aliasing. Validate runtime-dependent changes in-game and report
checks not run; DLL linking alone does not establish runtime safety.

Leave the SDK submodule untouched unless explicitly requested. Ask before changing
dependencies.

`Plugin::RUNTIME` defaults to `RuntimeVersion::RUNTIME_INDEPENDENT`. This does not
guarantee compatibility across game updates; native layouts and function addresses
can still change.

### Layout and conventions

`src/lib.rs` handles plugins; `invocable.rs`, `export.rs`, and `class.rs` define
calls and exports. `src/types/` and `src/systems/` wrap SDK types and runtime access.
`deps/RED4ext.SDK/` is the upstream submodule.

Branch names: `feat/`, `fix/`, `chore/`. Commits and PR titles:
`<prefix>: <imperative summary>`.
