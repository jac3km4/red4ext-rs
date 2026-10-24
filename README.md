# red4ext-rs

Rust wrapper around [RED4ext.SDK](https://github.com/WopsS/RED4ext.SDK) for
building Cyberpunk 2077 plugins.

[API documentation](https://jac3km4.github.io/red4ext-rs/red4ext_rs/index.html)
· [Generated game bindings](https://github.com/jac3km4/red4ext-rs-bindings)

## Requirements

Build on **Windows x64** with:

- Rust **1.95+**, targeting `x86_64-pc-windows-msvc`.
- Visual Studio C++ build tools (x64 MSVC) and the Windows SDK.
- CMake **3.21+** on `PATH`.
- LLVM/libclang (`LIBCLANG_PATH` can point to the directory containing `libclang.dll`).

Use an x64 developer shell. Linux supports formatting only; rely on Windows CI
for native checks rather than attempting cross-compilation.

## Quickstart

The root `red4ext-rs` crate is a Rust library used by plugins, not a plugin DLL
itself. Downstream plugins must declare `crate-type = ["cdylib"]` and export the
RED4ext entry points with `export_plugin_symbols!`, as below. The workspace's
`red4ext-smoke-plugin` fixture is a separate `cdylib` used to check DLL linking.

`Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
red4ext-rs = { git = "https://github.com/jac3km4/red4ext-rs", rev = "v0.10.0" }
```

This example pins the `v0.10.0` release; development API changes may differ from that revision.

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

Optional features (none enabled by default): `log` for the logging facade,
`chrono` and `time` for game-time conversions. See the API docs for invoking
functions and exporting classes; prefer generated game bindings to hand-written layouts.

## Contributing

After cloning:

```sh
git submodule update --init --recursive
```

`build.rs` compiles the SDK and generates bindings from `deps/wrapper.hpp` and
`deps/allowlist.txt`; never edit generated files in `target/`.

### Validation

Run from the root on Windows; keep `Cargo.lock` checked in.
Native checks can be slow; only need to run before pushing, not after every edit.

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

Verify FFI layouts and ownership against SDK source. Owning wrappers must release
allocations through the matching allocator, including custom `Drop` where needed.
Never free engine memory with Rust's allocator; validate runtime-dependent changes in-game.

`Plugin::RUNTIME` defaults to `RuntimeVersion::RUNTIME_INDEPENDENT`. This does not
guarantee compatibility across game updates; native layouts and function addresses
can still change.

### Build performance

- `RED4EXT_USE_PCH`: `1` (default) enables SDK precompiled headers; `0` disables them.
- `RED4EXT_BUILD_TIMINGS=1`: reports SDK and bindgen phase timings.
- Cargo's `--timings`: writes an HTML report to `target/cargo-timings/`.

Compare PCH on/off with the same toolchain and features. Measure clean and warm
builds separately; unchanged builds may skip the build script.

### Layout and conventions

`src/lib.rs` handles plugins; `invocable.rs`, `export.rs`, and `class.rs` define
calls and exports. `src/types/` and `src/systems/` wrap SDK types and runtime access.
`deps/RED4ext.SDK/` is the upstream submodule.

Branches: `feat/`, `fix/`, `chore/`. Commits and PR titles:
`<prefix>: <imperative summary>`.
