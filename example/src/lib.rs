#![feature(arbitrary_self_types)]
use red4ext_rs::prelude::*;

define_plugin! {
    name: "example",
    author: "author",
    version: 0:1:0,
    on_register: {
        register_function!("SumInts", sum_ints);
        register_function!("UseTypes", use_types);
        register_function!("CallDemo", call_demo);
    }
}

/// call function with primitives
///
/// try in-game in CET console:
///
/// ```lua
/// LogChannel(CName.new("DEBUG"), SumInts({ 2000, 77 }))
/// ```
fn sum_ints(ints: Vec<i32>) -> i32 {
    ints.iter().sum()
}

/// call function with game special types
///
/// try in-game in CET console:
///
/// ```lua
/// UseTypes(CName.new("Test"), TDBID.Create("Items.BlackLaceV0"), ItemID.FromTDBID(TDBID.Create("Items.BlackLaceV0")), Game.GetPlayer():GetEntityID(), "base\\characters\\entities\\player\\player_ma_fpp.ent", Game.GetTimeSystem():GetSimTime())
/// ```
/// > ⚠️ output can be found in mod's logs
fn use_types(
    name: CName,
    tweak: TweakDbId,
    item: ItemId,
    entity: EntityId,
    res: ResRef,
    sim: EngineTime,
) {
    info!("got CName {name:#?}, TweakDBID {tweak:#?}, ItemID {item:#?}, EntityID {entity:#?}, ResRef {res:#?}");
    let res = res_ref!("base" / "mod" / "custom.ent").unwrap();
    info!("created res ref: {res:#?}");
    info!("engine time: {:?} = {}", sim, EngineTime::to_float(sim));
}

/// call function with handle
///
/// try in-game in CET console:
///
/// ```lua
/// CallDemo(Game.GetPlayer())
/// ```
/// > ⚠️ output can be found in mod's logs
fn call_demo(player: Ref<PlayerPuppet>) {
    let res = add_u32(2, 2);
    info!("2 + 2 = {}", res);

    info!("player display name: {}", player.get_display_name());
    info!("player vehicles: {}", player.get_unlocked_vehicles_size());
    player.disable_camera_bobbing(true);
    let can_apply_breathing = PlayerPuppet::can_apply_breathing_effect(Ref::downgrade(&player));
    info!("can apply breating effect: {}", can_apply_breathing);
}

/// import a global operator
///
/// function names gets automatically mangled,
/// this one becomes `OperatorAdd;Uint32Uint32;Uint32`
///
/// try in-game in CET console:
///
/// ```lua
/// LogChannel(CName.new("DEBUG"), OperatorAdd(2000, 77))
/// ```
/// > ⚠️ output can be found in mod's logs
#[redscript_global(name = "OperatorAdd", operator)]
fn add_u32(l: u32, r: u32) -> u32;

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

    /// imports 'private func DisableCameraBobbing(b: Bool) -> Void'
    fn disable_camera_bobbing(self: &Ref<Self>, toggle: bool);

    /// imports 'public final static func CanApplyBreathingEffect(player: wref<PlayerPuppet>) -> Bool'
    fn can_apply_breathing_effect(player: WRef<PlayerPuppet>) -> bool;
}

impl ClassType for PlayerPuppet {
    // should be ScriptedPuppet if we were re-creating the entire class hierarchy,
    // but IScriptable can be used instead because every scripted class inherits from it
    type BaseClass = IScriptable;

    const NAME: &'static str = "PlayerPuppet";
}

/// define a binding for a native struct type
///
/// see [RED4ext.SDK](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/Scripting/Natives/Generated/EngineTime.hpp)
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
struct EngineTime {
    pub unk00: [u8; 8],
}

unsafe impl NativeRepr for EngineTime {
    const NAME: &'static str = "EngineTime";
}

#[redscript_import]
impl EngineTime {
    /// imports `public static native func ToFloat(self: EngineTime) -> Float`
    #[redscript(native)]
    fn to_float(time: EngineTime) -> f32;
}
