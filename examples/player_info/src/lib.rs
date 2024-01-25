#![feature(arbitrary_self_types)]
use red4ext_rs::prelude::*;

define_plugin! {
    name: "player_info",
    author: "author",
    version: 0:1:0,
    on_register: {
        register_function!("DumpPlayerInfo", dump_player_info);
    }
}

/// call function with handle
///
/// try in-game in CET console:
///
/// ```lua
/// DumpPlayerInfo(Game.GetPlayer())
/// ```
/// > ⚠️ output can be found in mod's logs
fn dump_player_info(player: Ref<PlayerPuppet>) {
    info!("player display name: {}", player.get_display_name());
    info!("player vehicles: {}", player.get_unlocked_vehicles_size());
    let can_apply_breathing = PlayerPuppet::can_apply_breathing_effect(Ref::downgrade(&player));
    info!("can apply breating effect: {}", can_apply_breathing);
    let class_name = Ref::get_class_name(player.clone());
    info!("class name: {}", class_name);
    let is_ent_entity = Ref::is_a(player.clone(), CName::new("entEntity"));
    info!("is an entEntity: {}", is_ent_entity);
    let is_exactly_a_player_puppet = Ref::is_exactly_a(player, CName::new(PlayerPuppet::NAME));
    info!("is exactly a PlayerPuppet: {}", is_exactly_a_player_puppet);
}

/// define a binding for a class type
#[derive(Debug)]
struct PlayerPuppet;

#[redscript_import]
impl PlayerPuppet {
    /// imports `public native func GetDisplayName() -> String`
    ///
    /// the method name is interpreted as PascalCase
    ///
    /// you can also specify it explicitly with a `name` attribute
    #[redscript(native)]
    fn get_display_name(self: &Ref<Self>) -> String;

    /// imports `private func GetUnlockedVehiclesSize() -> Int32`
    fn get_unlocked_vehicles_size(self: &Ref<Self>) -> i32;

    /// imports 'public final static func CanApplyBreathingEffect(player: wref<PlayerPuppet>) -> Bool'
    fn can_apply_breathing_effect(player: WRef<PlayerPuppet>) -> bool;
}

impl ClassType for PlayerPuppet {
    // should be ScriptedPuppet if we were re-creating the entire class hierarchy,
    // but IScriptable can be used instead because every scripted class inherits from it
    type BaseClass = IScriptable;

    const NAME: &'static str = "PlayerPuppet";
}
