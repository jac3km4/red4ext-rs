# red4ext-rs
Automagical Rust binding to [RED4ext](https://github.com/WopsS/RED4ext.SDK)

## usage
```rust
use red4ext_rs::prelude::*;

// this plugin registers a single function called SumInts
// you'll be able to refer to it from both redscript and CET
define_plugin! {
    name: "example",
    author: "jekky",
    version: 1:0:0,
    on_register: {
        register_function!("SumInts", sum_ints);
    }
}

fn sum_ints(ints: Vec<i32>) -> i32 {
    ints.iter().sum()
}
```
```swift
// this native definition will be required if you want to call it from redscript
native func SumInts(ints: array<Int32>) -> Int32;
```

A complete example project is available [here](https://github.com/jac3km4/red4ext-rs-example).

## macros
Additionally, you can enable the `macros` feature for some convenience import/export proc macros.
```rs
// import a global operator
// function names gets automatically mangled
// this one becomes OperatorAdd;Uint32Uint32;Uint32
#[redscript_global(name = "OperatorAdd", operator)]
fn add_u32(l: u32, r: u32) -> u32;

// define a binding for a class type
#[derive(Clone, Default)]
#[repr(transparent)]
struct PlayerPuppet(Ref<IScriptable>);

#[redscript_import]
impl PlayerPuppet {
    // imports 'public native func GetDisplayName() -> String'
    // the method name is interpreted as PascalCase
    // you can also specify it explicitly with a `name` attribute
    #[redscript(native)]
    fn get_display_name(&self) -> String;

    // imports 'private func DisableCameraBobbing(b: Bool) -> Void'
    fn disable_camera_bobbing(&self, toggle: bool) -> ();
}
```

## debugging
When compiled in debug mode, a panic handler is installed for each function. It helps with debugging common issues like function invokation errors:
```log
[2023-04-22 16:38:41.896] [example] [error] PrintDebug function panicked: OperatorAdd;Uint32Uint32;Uint32: ArgMismatch { expected: "Uint32", index: 1 }
```

## credits
- WopsS for [RED4ext](https://github.com/WopsS/RED4ext.SDK)
