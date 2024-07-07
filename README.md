# red4rs
Convenience Rust wrapper around [RED4ext.SDK](https://github.com/WopsS/RED4ext.SDK).

## usage

### set up a basic plugin
```rs
use red4rs::{
    export_plugin, log, systems::RttiRegistrator, wcstr, Plugin, PluginOps, SdkEnv, SemVer, U16CStr,
};

pub struct Example;

impl Plugin for Example {
    const NAME: &'static U16CStr = wcstr!("example");
    const AUTHOR: &'static U16CStr = wcstr!("jekky");
    const VERSION: SemVer = SemVer::new(0, 1, 0);

    fn on_init(env: &SdkEnv) {
        // we can use the env to write red4ext logs
        log::info!(env, "Hello world!");

        // we can request the RTTI to invoke our functions to do some setup
        RttiRegistrator::add(Some(register), Some(post_register));
    }
}

export_plugin!(Example);

unsafe extern "C" fn register() {
    // you can obtain the env anywhere as long as it's been initialized
    let env = Example::env();
    log::info!(env, "I'm registering!");
    // we will register types here
}

unsafe extern "C" fn post_register() {
    // we will register functions here
}
```

### expose a native function
```rust
use red4rs::{global, systems::RttiSystemMut};

unsafe extern "C" fn post_register() {
    // create your RTTI bindings first
    let global = global!(example).to_rtti(c"Example");

    // then acquire the RTTI for writing, you have to order it this way to avoid a deadlock
    let mut rtti = RttiSystemMut::get();
    // then register your stuff
    rtti.register_function(global);
}

fn example() -> String {
    "Hello world!".to_owned()
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
use red4rs::{
    global, method,
    systems::RttiSystemMut,
    types::{CName, IScriptable, Native, NativeClass, Ref, ScriptClass},
};

unsafe extern "C" fn register() {
    let mut rtti = RttiSystemMut::get();
    let parent = rtti.get_class(CName::new("IScriptable")).unwrap();
    let class = NativeClass::<MyClass>::new_handle(parent);
    rtti.register_class(class);
}

unsafe extern "C" fn post_register() {
    // create your RTTI bindings first
    let method = method!(MyClass::value).to_rtti(c"GetValue");
    let global = global!(example).to_rtti(c"Example");

    // then acquire the RTTI
    let mut rtti = RttiSystemMut::get();
    // then register your stuff
    rtti.get_class(CName::new("MyClass"))
        .unwrap()
        .add_method(method);
    rtti.register_function(global);
}

#[derive(Debug, Default, Clone)]
#[repr(C)]
struct MyClass {
    base: IScriptable,
    value: i32,
}

impl MyClass {
    fn value(&self) -> i32 {
        self.value
    }
}

unsafe impl ScriptClass for MyClass {
    const CLASS_NAME: &'static str = "MyClass";
    type Kind = Native;
}

fn example() -> Ref<MyClass> {
    Ref::<MyClass>::new_with(|t| t.value = 1337).unwrap()
}
```
...and on REDscript side:
```swift
native class MyClass {
    native func GetValue() -> Int32;
}

native func Example() -> ref<MyClass>;
```
