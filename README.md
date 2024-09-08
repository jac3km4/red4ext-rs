# red4ext-rs
Rust wrapper around [RED4ext.SDK](https://github.com/WopsS/RED4ext.SDK).

## documentation
Read the [documentation](https://jac3km4.github.io/red4ext-rs/red4ext_rs/index.html)!

## usage

### quickstart
Define your `Cargo.toml`:
```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"

[lib]
# we want to compile to a DLL
crate-type = ["cdylib"]

[dependencies]
red4ext-rs = { git = "https://github.com/jac3km4/red4ext-rs", features = ["log"], rev = "v0.8.1" }
# you can also add the bindings crate which exposes all in-game types for convenience
red4ext-rs-bindings = { git = "https://github.com/jac3km4/red4ext-rs-bindings", rev = "v0.4.1" }
```

### set up a basic plugin
```rs
use red4ext_rs::{
    export_plugin_symbols, exports, global, wcstr, Exportable, GlobalExport, Plugin, SemVer,
    U16CStr,
};

pub struct Example;

impl Plugin for Example {
    const AUTHOR: &'static U16CStr = wcstr!("me");
    const NAME: &'static U16CStr = wcstr!("example");
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

export_plugin_symbols!(Example);

fn add2(a: i32) -> i32 {
    a + 2
}
```

You can now build your project with `cargo build` and copy the compiled DLL from `{project}\target\debug\{project}.dll` to `{game}\red4ext\plugins\`. It should then be loaded by RED4ext and your function should be callable from REDscript and CET.

### call global and instance functions
```rust
use red4ext_rs::call;
use red4ext_rs::types::{IScriptable, Ref};

// you can expose Rust functions to the game as long as their signatures consist of supported
// types, you'll see a compiler error when you try to use an unsupported type like i128
fn example(player: Ref<IScriptable>) -> i32 {
    // the line below will attempt to look up a matching method in the instance and call it
    let size = call!(player, "GetDeviceActionMaxQueueSize;" () -> i32).unwrap();
    // the lines below will attempt to look up a matching static method (scripted or native) and call it
    let _ = call!("MathHelper"::"EulerNumber;"() -> f32).unwrap();
    let _ = call!("PlayerPuppet"::"GetCriticalHealthThreshold;" () -> f32).unwrap();
    // the line below invokes a global native function (the operator for adding two Int32)
    let added1 = call!("OperatorAdd;Int32Int32;Int32" (size, 4i32) -> i32).unwrap();
    added1
}
```

### interact with in-game scripted and native types using auto-generated bindings

See [red4ext-rs-bindings](https://github.com/jac3km4/red4ext-rs-bindings) for bindings for all
types defined in RTTI in the game.

### define and export your own class type
```rust
use std::cell::Cell;

use red4ext_rs::types::IScriptable;
use red4ext_rs::{class_kind, exports, methods, ClassExport, Exportable, ScriptClass};

// ...defined in impl Plugin
fn exports() -> impl Exportable {
    exports![ClassExport::<MyClass>::builder()
        .base("IScriptable")
        .methods(methods![
            c"GetValue" => MyClass::value,
            c"SetValue" => MyClass::set_value,
            event c"OnInitialize" => MyClass::on_initialize
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

    fn on_initialize(&self) {}
}

unsafe impl ScriptClass for MyClass {
    type Kind = class_kind::Native;

    const NAME: &'static str = "MyClass";
}
```
...and on REDscript side:
```swift
native class MyClass {
    native func GetValue() -> Int32;
    native func SetValue(a: Int32);
    native cb func OnInitialize();
}
```

### interact with scripted classes using hand-written bindings
```rust
use red4ext_rs::types::{EntityId, Ref};
use red4ext_rs::{class_kind, ScriptClass, ScriptClassOps};

#[repr(C)]
struct AddInvestigatorEvent {
    investigator: EntityId,
}

unsafe impl ScriptClass for AddInvestigatorEvent {
    type Kind = class_kind::Scripted;

    const NAME: &'static str = "AddInvestigatorEvent";
}

fn example() -> Ref<AddInvestigatorEvent> {
    // we can create new refs of script classes
    let instance = AddInvestigatorEvent::new_ref_with(|inst| {
        inst.investigator = EntityId::from(0xdeadbeef);
    })
    .unwrap();

    // we can obtain a reference to the fields of the ref
    let fields = unsafe { instance.fields() }.unwrap();
    let _investigator = fields.investigator;

    instance
}
```

### interact with native classes using hand-written bindings
```rust
use red4ext_rs::types::{IScriptable, Ref};
use red4ext_rs::{class_kind, ScriptClass, ScriptClassOps};

#[repr(C)]
struct ScanningEvent {
    base: IScriptable,
    state: u8,
}

unsafe impl ScriptClass for ScanningEvent {
    type Kind = class_kind::Native;

    const NAME: &'static str = "gameScanningEvent";
}

fn example() -> Ref<ScanningEvent> {
    ScanningEvent::new_ref_with(|inst| {
        inst.state = 1;
    })
    .unwrap()
}
```

### interact with native game systems
```rust
use red4ext_rs::types::{CName, EntityId, GameEngine, Opt};
use red4ext_rs::{call, RttiSystem};

fn example() {
    let rtti = RttiSystem::get();
    let class = rtti.get_class(CName::new("gameGameAudioSystem")).unwrap();
    let engine = GameEngine::get();
    let game = engine.game_instance();
    let system = game.get_system(class.as_type());
    call!(system, "Play" (CName::new("ono_v_pain_long"), Opt::<EntityId>::Default, Opt::<CName>::Default) -> ()).unwrap()
}
```
