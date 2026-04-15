use std::mem;

use super::{IScriptable, Ref, Type};
use crate::class::{ScriptClass, class_kind};
use crate::raw::root::RED4ext as red;
use crate::types::WeakRef;
use crate::types::rtti::IScriptableVft;
use crate::{NativeRepr, VoidPtr};

/// Scripted game instance.
///
/// It's worth noting that `GameInstance` is named after Redscript and Lua,
/// but it differs from [RED4ext naming convention](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/Scripting/Natives/ScriptGameInstance.hpp).
#[derive(Default)]
#[repr(transparent)]
pub struct GameInstance(red::ScriptGameInstance);

impl GameInstance {
    #[inline]
    pub fn new() -> Self {
        Self(unsafe {
            red::ScriptGameInstance::new(GameEngine::get().game_instance() as *const _ as *mut _)
        })
    }
}

unsafe impl NativeRepr for GameInstance {
    const NAME: &'static str = "ScriptGameInstance";
}

/// Native game instance.
///
/// Please note that it differs from Redscript and Lua's `GameInstance`,
/// see [`GameInstance`].
#[derive(Default)]
#[repr(transparent)]
pub struct NativeGameInstance(red::GameInstance);

impl NativeGameInstance {
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

impl Drop for NativeGameInstance {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.vft().destroy)(self) };
    }
}

#[repr(C)]
pub struct GameInstanceVft {
    destroy: unsafe extern "C" fn(this: *mut NativeGameInstance),
    get_system:
        unsafe extern "C" fn(this: *const NativeGameInstance, ty: &Type) -> *mut red::IScriptable,
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

#[repr(transparent)]
pub struct GameEngine(red::CGameEngine);

impl GameEngine {
    pub fn get<'a>() -> &'a Self {
        unsafe { &*(red::CGameEngine::Get() as *const GameEngine) }
    }

    pub fn game_instance(&self) -> &NativeGameInstance {
        unsafe { &*((*self.0.framework).gameInstance as *const NativeGameInstance) }
    }
}

#[repr(transparent)]
pub struct ScriptableSystem(red::ScriptableSystem);

unsafe impl ScriptClass for ScriptableSystem {
    type Kind = class_kind::Native;

    const NAME: &'static str = "gameScriptableSystem";
}

impl AsRef<IScriptable> for ScriptableSystem {
    fn as_ref(&self) -> &IScriptable {
        unsafe { mem::transmute(&self.0._base._base) }
    }
}

#[repr(transparent)]
pub struct IUpdatableSystem(IScriptable);

unsafe impl ScriptClass for IUpdatableSystem {
    type Kind = class_kind::Native;

    const NAME: &'static str = "IUpdatableSystem";
}

impl IUpdatableSystem {
    #[inline]
    pub fn vft(&self) -> &IUpdatableSystemVft {
        unsafe { mem::transmute::<&IScriptableVft, &IUpdatableSystemVft>(self.0.vft()) }
    }
}

impl AsRef<IScriptable> for IUpdatableSystem {
    fn as_ref(&self) -> &IScriptable {
        unsafe { mem::transmute(&self.0) }
    }
}

#[repr(C)]
pub struct IGameSystem {
    base: IUpdatableSystem,
    game_instance: *mut NativeGameInstance,
}

unsafe impl ScriptClass for IGameSystem {
    type Kind = class_kind::Native;

    const NAME: &'static str = "gameIGameSystem";
}

impl IGameSystem {
    #[inline]
    pub fn vft(&self) -> &IGameSystemVft {
        unsafe { mem::transmute::<&IUpdatableSystemVft, &IGameSystemVft>(self.base.vft()) }
    }
}

impl AsRef<IScriptable> for IGameSystem {
    fn as_ref(&self) -> &IScriptable {
        unsafe { mem::transmute(&self.base.0) }
    }
}

#[repr(C)]
pub struct IUpdatableSystemVft {
    base: IScriptableVft,
    on_register_updates: unsafe extern "C" fn(this: VoidPtr, registrar: VoidPtr),
}

#[repr(C)]
pub struct IGameSystemVft {
    base: IUpdatableSystemVft,
    on_world_attached: VoidPtr,
    on_before_world_detach: VoidPtr,
    on_world_detached: VoidPtr,
    on_after_world_detach: VoidPtr,
    on_before_game_save: VoidPtr,
    on_game_save: VoidPtr,
    on_after_game_save: VoidPtr,
    on_game_load: VoidPtr,
    on_game_restored: VoidPtr,
    on_game_prepared: VoidPtr,
    on_game_paused: VoidPtr,
    on_game_resumed: VoidPtr,
    is_saving_locked: VoidPtr,
    on_streaming_world_loaded: VoidPtr,
    sub_180: VoidPtr,
    sub_188: VoidPtr,
    sub_190: VoidPtr,
    sub_198: VoidPtr,
    on_initialize: VoidPtr,
    on_uninitialize: VoidPtr,
}
