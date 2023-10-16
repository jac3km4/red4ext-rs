use red4ext_rs::prelude::*;

define_plugin! {
    name: "hello_world",
    author: "author",
    version: 0:1:0,
    on_register: {
        register_function!("HelloWorld", hello_world);
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
