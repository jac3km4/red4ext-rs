# red4ext-rs
Automagical Rust binding to [RED4ext](https://github.com/WopsS/RED4ext.SDK)

```rust
use red4ext_rs::prelude::*;

#[redscript_export]
fn to_lowercase(str: String) -> String {
    str.to_lowercase()
}

#[redscript_export]
fn on_menu_load(controller: Ref<RED4ext::IScriptable>) {
    // calling game functions
    call!("Max" (1i32, 2i32) -> i32);
    call!("RoundMath;Float" (1.2f32) -> i32);
    call!(controller, "Size" () -> i32);
}

#[ctor::ctor]
fn init() {
    on_register(register, post_register);
}

extern "C" fn register() {}

extern "C" fn post_register() {
    register_function!("ToLowercase", to_lowercase);
    register_function!("OnMainMenuLoadTest", on_menu_load);
}
```

```swift
native func ToLowercase(param: String) -> String;
native func OnMainMenuLoadTest(controller: ref<ListController>);
```

## credits
- WopsS for [RED4ext](https://github.com/WopsS/RED4ext.SDK)
