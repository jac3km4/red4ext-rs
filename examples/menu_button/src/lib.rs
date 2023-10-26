#![feature(arbitrary_self_types)]
use red4ext_rs::prelude::*;

define_plugin! {
    name: "menu_button",
    author: "author",
    version: 0:1:0,
    on_register: {
        register_function!("CustomizeMenu", customize_menu);
    }
}

/// here we expose a function that adds a button to the main menu,
/// it's invoked from a method wrapper in redscript
fn customize_menu(menu: Ref<SingleplayerMenuGameController>) {
    menu.add_menu_item(
        ScriptRef::new(&mut "Dummy Button!".into()),
        CName::new("OnBuyGame"),
    );
}

/// define a binding for a class type
#[derive(Debug)]
struct SingleplayerMenuGameController;

#[redscript_import]
impl SingleplayerMenuGameController {
    /// imports `protected final func AddMenuItem(const label: script_ref<String>, spawnEvent: CName) -> Void`
    fn add_menu_item(self: &Ref<Self>, label: ScriptRef<'_, RedString>, event: CName);
}

impl ClassType for SingleplayerMenuGameController {
    type BaseClass = IScriptable;

    const NAME: &'static str = "SingleplayerMenuGameController";
}
