# red4ext-rs
Automagical Rust binding to [RED4ext](https://github.com/WopsS/RED4ext.SDK)

```rust
use red4ext_rs::prelude::*;

#[redscript]
fn pow5(a: i32) -> i32 {
    a.pow(5)
}

#[ctor::ctor]
fn init() {
    ...
    register_function!(pow5);
}
```

```swift
native func pow5(param: Int32) -> Int32;
```

## credits
- WopsS for [RED4ext](https://github.com/WopsS/RED4ext.SDK)
