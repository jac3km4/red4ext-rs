# red4rs
Convenience Rust wrapper around [RED4ext.SDK](https://github.com/WopsS/RED4ext.SDK).

## usage

### set up a basic plugin
```rs
use red4rs::{
    export_plugin, exports, global, wcstr, Exportable, GlobalExport, Plugin, SemVer, U16CStr,
};

pub struct Example;

impl Plugin for Example {
    const NAME: &'static U16CStr = wcstr!("example");
    const AUTHOR: &'static U16CStr = wcstr!("jekky");
    const VERSION: SemVer = SemVer::new(0, 1, 0);

    // exports a named global function
    fn exports() -> impl Exportable {
        exports![
            GlobalExport(global!(c"Add2", add2)),
            // you can export global functions and classes
            // ClassExport::<MyClass>::builder()
            //     .base("IScriptable")
            //     .methods(methods![
            //         c"GetValue" => MyClass::value,
            //         c"SetValue" => MyClass::set_value,
            //     ])
            //     .build()
        ]
    }
}

export_plugin!(Example);

fn add2(a: i32) -> i32 {
    a + 2
}
```

### call global and instance functions
```rust
use red4rs::{
    call,
    types::{IScriptable, Ref},
};

fn example(player: Ref<IScriptable>) -> i32 {
    let size = call!(player.instance().unwrap(), "GetDeviceActionMaxQueueSize;" () -> i32).unwrap();
    let added1 = call!("OperatorAdd;Int32Int32;Int32" (size, 4i32) -> i32).unwrap();
    added1
}
```

### instantiate and interact with scripted classes
```rust
use red4rs::types::{EntityId, Ref, ScriptClass, Scripted};

#[repr(C)]
struct AddInvestigatorEvent {
    investigator: EntityId,
}

unsafe impl ScriptClass for AddInvestigatorEvent {
    const CLASS_NAME: &'static str = "AddInvestigatorEvent";
    type Kind = Scripted;
}

fn example() -> Ref<AddInvestigatorEvent> {
    Ref::<AddInvestigatorEvent>::new_with(|inst| {
        inst.investigator = EntityId::from(0xdeadbeef);
    })
    .unwrap()
}
```

### instantiate and interact with native classes
```rust
use red4rs::types::{IScriptable, Native, Ref, ScriptClass};

#[repr(C)]
struct ScanningEvent {
    base: IScriptable,
    state: u8,
}

unsafe impl ScriptClass for ScanningEvent {
    const CLASS_NAME: &'static str = "ScanningEvent";
    const NATIVE_NAME: &'static str = "gameScanningEvent";
    type Kind = Native;
}

fn example() -> Ref<ScanningEvent> {
    Ref::<ScanningEvent>::new_with(|inst| {
        inst.state = 1;
    })
    .unwrap()
}
```


### define a custom class type
```rust
use std::cell::Cell;

use red4rs::{
    exports, methods,
    types::{IScriptable, Native, ScriptClass},
    ClassExport, Exportable,
};

// ...defined in impl Plugin
fn exports() -> impl Exportable {
    exports![ClassExport::<MyClass>::builder()
        .base("IScriptable")
        .methods(methods![
            c"GetValue" => MyClass::value,
            c"SetValue" => MyClass::set_value,
        ])
        .build(),]
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
struct MyClass {
    base: IScriptable,
    value: Cell<i32>,
}

impl MyClass {
    fn value(&self) -> i32 {
        self.value.get()
    }

    fn set_value(&self, value: i32) {
        self.value.set(value);
    }
}

unsafe impl ScriptClass for MyClass {
    const CLASS_NAME: &'static str = "MyClass";
    type Kind = Native;
}
```
...and on REDscript side:
```swift
native class MyClass {
    native func GetValue() -> Int32;
    native func SetValue(a: Int32);
}
```
