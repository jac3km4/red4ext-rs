# red4ext-rs
Automagical Rust binding to [RED4ext](https://github.com/WopsS/RED4ext.SDK).

## quickstart
Modify `Cargo.toml` to make your crate a `cdylib` so that it compiles into a DLL:
```toml
[lib]
crate-type = ["cdylib"]
```
Define your plugin in `src/lib.rs`:
```rust
use red4ext_rs::prelude::*;

// this macro generates boilerplate that allows red4ext to boostrap the plugin
define_plugin! {
    name: "example",
    author: "author",
    version: 0:1:0,
    on_register: {
        // functions registered here become accessible in redscript and CET under the name provided as the first parameter
        register_function!("SumInts", sum_ints);
    }
}

fn sum_ints(ints: Vec<i32>) -> i32 {
    ints.iter().sum()
}
```
If you want the function to be available in redscript you need to provide a binding in redscript too:
```swift
native func SumInts(ints: array<Int32>) -> Int32;
```
Now, when you run `cargo build --release`, a DLL file will be generated in `target/release`.
This DLL is a plugin that is ready to be deployed to `Cyberpunk 2077/red4ext/plugins/`.

A complete example project is available [here](https://github.com/jac3km4/red4ext-rs-example).

## calling functions
The main crate exposes small macro that allows you to call game functions directly from Rust:
```rs
let result = call!("OperatorAdd;Uint32Uint32;Uint32" (2u32, 2u32) -> u32);
```
It can also be used to invoke methods on objects:
```rs
fn is_player(scriptable: Ref<IScriptable>) -> bool {
    call!(scriptable, "IsPlayer;" () -> bool)
}
```
It works OK if you don't need to invoke game functions frequently, but for larger projects a more
convenient, [proc macro](#proc-macros) approach is described in the next section.

## proc macros
The `macros` crate feature enables a few proc macros that make interop even easier.

Available macros:
- `redscript_global`
  
  Imports a global and exposes it as plain a Rust function,
  taking care of name mangling automatically.

  Parameters:
    - `name` - the in-game function name (it defaults to a PascalCase version of the Rust name)
    - `native` - whether the function is native (affects mangling)
    - `operator` - whether the function is an operator (affects mangling)
  
  Example:
    ```rs
    #[redscript_global(name = "OperatorAdd", operator)]
    fn add_u32(l: u32, r: u32) -> u32;
    ```
- `redscript_import`

  Imports a set of methods for a class type. The `Self` type has to be a struct with a single `Ref<IScriptable>` member.

  Parameters (optionally specified for each method with the `#[redscript(...)]` attribute):
    - `name` - the in-game function name (it defaults to a PascalCase version of the Rust name)
    - `native` - whether the function is native (affects mangling)
    - `cb` - whether the function is a callback (affects mangling)
  
  Example:
    ```rs
    #[derive(Clone, Default)]
    #[repr(transparent)]
    struct PlayerPuppet(Ref<IScriptable>);

    #[redscript_import]
    impl PlayerPuppet {
        // imports 'public native func GetDisplayName() -> String'
        #[redscript(native)]
        fn get_display_name(&self) -> String;

        // imports 'private func DisableCameraBobbing(b: Bool) -> Void'
        #[redscript(name = "DisableCameraBobbing")]
        fn disable_cam_bobbing(&self, toggle: bool) -> ();
    }
    ```

## custom types
By default this project only provides support for standard types like integers, floats and some collections.
If you want to use other types, you have to write your own binding which is relatively easy to do,
but it's on you to guarantee that it matches the layout of the underlying type.
- if you have types that directly map into one of the known primitives like `i32`, `String` etc.
  you should implement the `FromRepr` and `IntoRepr` traits for them;
  this is the only option that doesn't involve unsafe code
- **structs** should be represented as Rust structs with `#[repr(C)]`
    ```rs
    #[repr(C)]
    struct Vector2 {
        x: f32,
        y: f32,
    }

    unsafe impl NativeRepr for Vector2 {
        // this needs to refer to an actual in-game type name
        const NAME: &'static str = "Vector2";
    }
    ```
- **classes** should be represented as wrappers around `Ref<IScriptable>`
    ```rs
    #[repr(transparent)]
    struct PlayerPuppet(Ref<IScriptable>);

    unsafe impl NativeRepr for PlayerPuppet {
        const NAME: &'static str = "handle:PlayerPuppet";
    }
    ```
- **enums** should be represented as Rust enums with `#[repr(i64)]`
    ```rs
    #[repr(i64)]
    enum ShapeVariant {
        Fill = 0,
        Border = 1,
        FillAndBorder = 2,
    }

    unsafe impl NativeRepr for ShapeVariant {
        const NAME: &'static str = "inkEShapeVariant";
    }
    ```

## debugging
When compiled in debug mode, a panic handler is installed for each function. It helps with debugging common issues like function invokation errors:
```log
[2023-04-24 23:37:11.396] [example] [error] CallDemo function panicked: failed to invoke OperatorAdd;Uint32Uint32;Uint32: expected Uint32 argument at index 0
```

## credits
- WopsS for [RED4ext](https://github.com/WopsS/RED4ext.SDK)
