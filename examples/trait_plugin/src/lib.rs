use red4ext_rs::prelude::*;

define_trait_plugin! {
    name: "MyPlugin",
    author: "Me",
    plugin: MyPlugin
}

struct MyPlugin;

impl Plugin for MyPlugin {
    const VERSION: Version = Version::new(1, 0, 0);

    fn post_register() {
        register_function!("HelloWorld", hello_world);
    }

    fn is_version_independent() -> bool {
        false
    }
}

/// try in-game in CET console:
///
/// ```lua
/// HelloWorld()
/// ```
/// > ⚠️ output can be found in mod's logs
fn hello_world() {
    info!("Hello World!");
}
