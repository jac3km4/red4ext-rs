# How to

### set up a basic plugin
```rs
struct Example;

impl Plugin for Example {
    const NAME: &'static U16CStr = wcstr!("example");
    const AUTHOR: &'static U16CStr = wcstr!("jekky");
    const VERSION: SemVer = SemVer::new(0, 1, 0);

    fn on_init(env: &SdkEnv) {
        log::info!(env, "Hello world!");
    }
}

export_plugin!(Example);
```

### expose a native function
```rs
...
    fn on_init(_env: &SdkEnv) {
        RttiRegistrator::add(None, Some(post_register));
    }
}

unsafe extern "C" fn post_register() {
    // register your function
    RttiSystem::get().register_function(invocable!(example).to_global(c"Example"));
}


fn example() {
    let env = Example::env();
    log::info!(env, "Hello from example!");
}
```

### accept and return compatible types
```rs
unsafe extern "C" fn post_register() {
    // register your function
    RttiSystem::get().register_function(invocable!(example).to_global(c"Example"));
}


fn example(player: Ref<IScriptable>) -> String {
    player.instance().unwrap().class().name().as_str().to_owned()
}
```

### call global and instance functions
```rs
fn example(player: Ref<IScriptable>) {
    let env = Example::env();

    let res = call!("OperatorAdd;Int32Int32;Int32" (6i32, 4i32) -> i32);
    log::info!(env, "Add result: {:?}", res);

    let size = call!(player.instance().unwrap(), "GetDeviceActionMaxQueueSize;" () -> i32);
    log::info!(env, "GetDeviceActionMaxQueueSize result: {:?}", size);
}
```

### instantiate and interact with scripted classes
```rs
#[repr(C)]
struct AddInvestigatorEvent {
    investigator: EntityId,
}

unsafe impl ScriptClass for AddInvestigatorEvent {
    const CLASS_NAME: &'static str = "AddInvestigatorEvent";
    type Kind = Scripted;
}

fn example() -> Ref<AddInvestigatorEvent> {
    let env = Example::env();

    let inst = Ref::<AddInvestigatorEvent>::new_with(|inst| {
        inst.investigator = EntityId::from(0xdeadbeef);
    })
    .unwrap();
    log::info!(env, "{:?}", inst.fields().unwrap().investigator);

    inst
}
```

### instantiate and interact with native classes
```rs
#[repr(C)]
struct ScanningEvent {
    base: IScriptable,
    state: u8,
}

impl AsRef<IScriptable> for ScanningEvent {
    fn as_ref(&self) -> &IScriptable {
        &self.base
    }
}

impl AsMut<IScriptable> for ScanningEvent {
    fn as_mut(&mut self) -> &mut IScriptable {
        &mut self.base
    }
}

unsafe impl ScriptClass for ScanningEvent {
    const CLASS_NAME: &'static str = "ScanningEvent";
    const NATIVE_NAME: &'static str = "gameScanningEvent";
    type Kind = Native;
}

fn example() -> Ref<ScanningEvent> {
    let env = Example::env();

    let inst = Ref::<ScanningEvent>::new_with(|inst| {
        inst.state = 1;
    })
    .unwrap();
    log::info!(env, "state: {}", inst.fields().unwrap().state);

    inst
}
```
