use red4ext_rs as sdk;

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[repr(C)]
struct Example {
    base: sdk::types::IScriptable,
}

// The native base is first and there are no additional fields.
unsafe impl sdk::ScriptClass for Example {
    type Kind = sdk::class_kind::Native;

    const NAME: &'static str = "MacroCompilationExample";
}

impl Example {
    fn value(&self) -> i32 {
        42
    }
}

sdk::hooks! {
    static ADD_HOOK: fn(a: i32, b: i32) -> i32;
}

#[test]
fn qualified_macros_need_no_macro_imports() {
    // Multiple exports and a trailing comma exercise recursive expansion.
    let _exports = sdk::exports![
        sdk::GlobalExport(sdk::global!(c"Add", add)),
        sdk::GlobalExport(sdk::global!(c"OtherAdd", add)),
    ];
    let _method = sdk::method!(c"Value", Example::value);
    let methods = sdk::methods![c"Value" => Example::value];
    let static_methods = sdk::static_methods![c"Add" => Example::add];
    assert_eq!(methods.len(), 1);
    assert_eq!(static_methods.len(), 1);
    // Read the static pointer without attaching a hook or accessing the game.
    assert!(!unsafe { ADD_HOOK }.is_null());
}

impl Example {
    fn add(a: i32, b: i32) -> i32 {
        add(a, b)
    }
}
