# red4ext-rs
Automagical Rust binding to [RED4ext](https://github.com/WopsS/RED4ext.SDK)

```rust
use red4ext_rs::prelude::*;

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
native func SumInts(ints: array<Int32>) -> Int32;
```

## credits
- WopsS for [RED4ext](https://github.com/WopsS/RED4ext.SDK)
