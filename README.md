# red4ext-rs
Automagical Rust binding to [RED4ext](https://github.com/WopsS/RED4ext.SDK)

## usage
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

A complete example project is available [here](https://github.com/jac3km4/red4ext-rs-example).

## debugging
When compiled in debug mode, a panic handler is installed for each function. It helps with debugging common issues like function invokation errors:
```log
[2023-04-22 16:38:41.896] [example] [error] PrintDebug function panicked: OperatorAdd;Uint32Uint32;Uint32: ArgMismatch { expected: "Uint32", index: 1 }
```

## credits
- WopsS for [RED4ext](https://github.com/WopsS/RED4ext.SDK)
