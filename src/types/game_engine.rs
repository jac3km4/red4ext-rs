use std::mem;

use super::{IScriptable, Native, Ref, ScriptClass, Type};
use crate::raw::root::RED4ext as red;
use crate::types::WeakRef;
use crate::{NativeRepr, VoidPtr};

/// Scripted game instance.
///
/// Please note that `ScriptGameInstance` is called `GameInstance` in Redscript and Lua,
/// but here it follows [RED4ext naming convention](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/Scripting/Natives/ScriptGameInstance.hpp).
#[derive(Default)]
#[repr(transparent)]
pub struct ScriptGameInstance(red::ScriptGameInstance);

impl ScriptGameInstance {
    #[inline]
    pub fn new() -> Self {
        Self(unsafe {
            red::ScriptGameInstance::new(GameEngine::get().game_instance() as *const _ as *mut _)
        })
    }
}

unsafe impl NativeRepr for ScriptGameInstance {
    const NAME: &'static str = "ScriptGameInstance";
}

/// Native game instance.
///
/// Please note that it differs from Redscript and Lua's `GameInstance`,
/// see [`ScriptGameInstance`].
#[derive(Default)]
#[repr(transparent)]
pub struct GameInstance(red::GameInstance);

impl GameInstance {
    #[inline]
    pub fn get_system(&self, ty: &Type) -> Ref<ScriptableSystem> {
        let instance = unsafe { (self.vft().get_system)(self, ty) };
        if instance.is_null() {
            return Ref::default();
        }
        let instance: &WeakRef<ScriptableSystem> =
            unsafe { mem::transmute(&(*instance)._base.ref_) };
        instance.clone().upgrade().unwrap_or_default()
    }

    #[inline]
    fn vft(&self) -> &GameInstanceVft {
        unsafe { &*(self.0.vtable_ as *const GameInstanceVft) }
    }
}

impl Drop for GameInstance {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.vft().destroy)(self) };
    }
}

#[repr(C)]
pub struct GameInstanceVft {
    destroy: unsafe extern "fastcall" fn(this: *mut GameInstance),
    get_system:
        unsafe extern "fastcall" fn(this: *const GameInstance, ty: &Type) -> *mut red::IScriptable,
    _unk10: VoidPtr,
    _unk18: VoidPtr,
    _unk20: VoidPtr,
    _unk28: VoidPtr,
    _unk30: VoidPtr,
    _unk38: VoidPtr,
    _unk40: VoidPtr,
    _unk48: VoidPtr,
    _unk50: VoidPtr,
    _unk58: VoidPtr,
    _unk60: VoidPtr,
    _unk68: VoidPtr,
}

/// game engine.
#[repr(transparent)]
pub struct GameEngine(red::CGameEngine);

impl GameEngine {
    pub fn get<'a>() -> &'a Self {
        unsafe { &*(red::CGameEngine::Get() as *const GameEngine) }
    }

    pub fn game_instance(&self) -> &GameInstance {
        unsafe { &*((*self.0.framework).gameInstance as *const GameInstance) }
    }
}

#[repr(transparent)]
pub struct ScriptableSystem(red::ScriptableSystem);

unsafe impl ScriptClass for ScriptableSystem {
    type Kind = Native;

    const CLASS_NAME: &'static str = "gameScriptableSystem";
}

impl AsRef<IScriptable> for ScriptableSystem {
    fn as_ref(&self) -> &IScriptable {
        unsafe { mem::transmute(&self.0._base._base) }
    }
}
