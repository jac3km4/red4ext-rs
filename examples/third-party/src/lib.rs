#![feature(arbitrary_self_types)]
use red4ext_rs::prelude::*;

#[cfg(feature = "codeware")]
mod board;
#[cfg(feature = "codeware")]
mod reflection;

define_plugin! {
    name: "example",
    author: "author",
    version: 0:1:0,
    on_register: {
        register_function!("GetToggleFireMode", get_toggle_fire_mode);
    }
}

/// call function with third-party library
///
/// try in-game in CET console:
///
/// ```lua
/// LogChannel(CName.new("DEBUG"), GetToggleFireMode(Game.GetPlayer()));
/// ```
/// > ⚠️ requires Codeware v1.4.0+ and Cargo feature flag 'codeware'
#[cfg(feature = "codeware")]
fn get_toggle_fire_mode(player: Ref<PlayerPuppet>) -> bool {
    let board = player.get_player_state_machine_blackboard();
    let pin = player.toggle_fire_mode();
    let value = board.get_bool(pin.clone());
    value
}

#[cfg(not(feature = "codeware"))]
fn get_toggle_fire_mode() -> bool {
    info!("please install latest Codeware v1.4.0+ and use feature flag 'codeware'");
    false
}

/// define a binding for a class type
#[derive(Debug)]
struct PlayerPuppet;

#[cfg(feature = "codeware")]
#[redscript_import]
impl PlayerPuppet {
    /// `public func GetPlayerStateMachineBlackboard() -> IBlackboard`
    pub fn get_player_state_machine_blackboard(self: &Ref<Self>) -> Ref<crate::board::IBlackboard>;
}
#[cfg(feature = "codeware")]
impl PlayerPuppet {
    pub fn toggle_fire_mode(self: &Ref<Self>) -> crate::board::BlackboardIdBool {
        use crate::board::get_all_blackboard_defs;
        get_all_blackboard_defs()
            .player_state_machine()
            .toggle_fire_mode()
    }
}

impl ClassType for PlayerPuppet {
    // should be ScriptedPuppet if we were re-creating the entire class hierarchy,
    // but IScriptable can be used instead because every scripted class inherits from it
    type BaseClass = IScriptable;

    const NAME: &'static str = "PlayerPuppet";
}
