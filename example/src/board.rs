use red4ext_rs::prelude::{redscript_global, redscript_import, ClassType, NativeRepr};
use red4ext_rs::types::{CName, IScriptable, Ref, VariantExt};

use crate::reflection::Reflection;

/// `public static native func GetAllBlackboardDefs() -> AllBlackboardDefinitions`
#[redscript_global(native)]
pub fn get_all_blackboard_defs() -> Ref<AllBlackboardDefinitions>;

#[derive(Debug)]
pub struct AllBlackboardDefinitions;

impl ClassType for AllBlackboardDefinitions {
    type BaseClass = IScriptable;

    const NAME: &'static str = "gamebbAllScriptDefinitions";
}

impl AllBlackboardDefinitions {
    pub fn player_state_machine(self: &Ref<Self>) -> Ref<PlayerStateMachineDef> {
        let cls = Reflection::get_class(CName::new(Self::NAME))
            .into_ref()
            .expect("get class AllBlackboardDefinitions");
        let field = cls
            .get_property(CName::new("PlayerStateMachine"))
            .into_ref()
            .expect("get prop PlayerStateMachine on class AllBlackboardDefinitions");
        VariantExt::try_take(
            &mut field.get_value(VariantExt::new(red4ext_rs::prelude::Ref::<
                AllBlackboardDefinitions,
            >::downgrade(&self))),
        )
        .expect("prop PlayerStateMachine of type PlayerStateMachineDef")
    }
}

#[derive(Debug)]
pub struct PlayerStateMachineDef;

impl ClassType for PlayerStateMachineDef {
    type BaseClass = IScriptable;

    const NAME: &'static str = "PlayerStateMachineDef";
}

impl PlayerStateMachineDef {
    pub fn toggle_fire_mode(self: &Ref<Self>) -> BlackboardIdBool {
        let cls = crate::reflection::Reflection::get_class(CName::new(Self::NAME))
            .into_ref()
            .expect("get class PlayerStateMachineDef");
        let field = cls
            .get_property(CName::new("ToggleFireMode"))
            .into_ref()
            .expect("get prop ToggleFireMode for class PlayerStateMachineDef");
        VariantExt::try_take(
            &mut field.get_value(VariantExt::new(red4ext_rs::prelude::Ref::<
                PlayerStateMachineDef,
            >::downgrade(&self))),
        )
        .expect("prop ToggleFireMode of type BlackboardID_Bool")
    }
}

#[derive(Default, Clone)]
#[repr(C)]
pub struct Id {
    pub g: CName,
}

unsafe impl NativeRepr for Id {
    const NAME: &'static str = "gamebbID";
}

#[derive(Default, Clone)]
#[repr(C)]
pub struct BlackboardId {
    pub unk00: [u8; 8],
    pub none: Id,
}

unsafe impl NativeRepr for BlackboardId {
    const NAME: &'static str = "BlackboardID";
    const NATIVE_NAME: &'static str = "gamebbScriptID";
}

#[derive(Default, Clone)]
#[repr(C)]
pub struct BlackboardIdBool(BlackboardId);

unsafe impl NativeRepr for BlackboardIdBool {
    const NAME: &'static str = "BlackboardID_Bool";
    const NATIVE_NAME: &'static str = "gamebbScriptID_Bool";
}

#[derive(Debug, Default)]
pub struct IBlackboard;

impl ClassType for IBlackboard {
    type BaseClass = IScriptable;

    const NAME: &'static str = "gameIBlackboard";
}

#[redscript_import]
impl IBlackboard {
    /// `public native GetBool(id: BlackboardID_Bool): Bool`
    #[redscript(native)]
    pub fn get_bool(self: &Ref<Self>, id: BlackboardIdBool) -> bool;
}
