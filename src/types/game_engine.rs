use std::mem;

use super::{IScriptable, Ref, Type};
use crate::class::{ScriptClass, class_kind};
use crate::raw::root::RED4ext as red;
use crate::types::WeakRef;
use crate::types::rtti::IScriptableVft;
use crate::{NativeRepr, VoidPtr};

/// REDscript `GameInstance`, not the native [`NativeGameInstance`].
///
/// SDK: [ScriptGameInstance](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Natives/ScriptGameInstance.hpp).
#[derive(Default)]
#[repr(transparent)]
pub struct GameInstance(red::ScriptGameInstance);

impl GameInstance {
    /// Wraps the current native game instance for use in REDscript calls.
    ///
    /// Requires a live engine, game framework, and native game instance.
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

/// Native game instance,
/// not REDscript's [`GameInstance`]. Access through [`GameEngine::game_instance`].
/// This is a borrowed engine object, not a constructible or owning Rust value.
///
/// SDK: [GameInstance](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/GameEngine.hpp).
#[repr(transparent)]
pub struct NativeGameInstance(red::GameInstance);

impl NativeGameInstance {
    /// Looks up a system by its RTTI type and acquires a strong reference.
    ///
    /// Returns a null [`Ref`] if the system is absent or its weak reference has expired.
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

/// Native virtual table layout for [`NativeGameInstance`].
#[repr(C)]
pub struct GameInstanceVft {
    _destroy: unsafe extern "C" fn(this: *mut NativeGameInstance),
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
/// Game engine singleton.
/// Access requires a live engine and game framework.
///
/// SDK: [CGameEngine](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/GameEngine.hpp).
pub struct GameEngine(red::CGameEngine);

impl GameEngine {
    /// Borrows the engine singleton, which must already be initialized and remain live.
    pub fn get<'a>() -> &'a Self {
        unsafe { &*(red::CGameEngine::Get() as *const GameEngine) }
    }

    /// Borrows the native game instance from the initialized game framework.
    ///
    /// The framework and game instance must be live; this method does not check for null.
    pub fn game_instance(&self) -> &NativeGameInstance {
        unsafe { &*((*self.0.framework).gameInstance as *const NativeGameInstance) }
    }
}

#[repr(transparent)]
/// Scriptable game system.
///
/// SDK: [game::ScriptableSystem](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Natives/Generated/game/ScriptableSystem.hpp).
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
/// Updatable engine system.
///
/// SDK: [IUpdatableSystem](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Natives/IUpdatableSystem.hpp).
pub struct IUpdatableSystem(IScriptable);

unsafe impl ScriptClass for IUpdatableSystem {
    type Kind = class_kind::Native;

    const NAME: &'static str = "IUpdatableSystem";
}

impl IUpdatableSystem {
    #[inline]
    /// Borrows the system's native virtual table.
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
/// Game system interface.
///
/// SDK: [game::IGameSystem](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Natives/gameIGameSystem.hpp).
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
    /// Borrows the system's native virtual table.
    pub fn vft(&self) -> &IGameSystemVft {
        unsafe { mem::transmute::<&IUpdatableSystemVft, &IGameSystemVft>(self.base.vft()) }
    }
}

impl AsRef<IScriptable> for IGameSystem {
    fn as_ref(&self) -> &IScriptable {
        unsafe { mem::transmute(&self.base.0) }
    }
}

/// Native virtual table layout for [`IUpdatableSystem`].
#[repr(C)]
pub struct IUpdatableSystemVft {
    base: IScriptableVft,
    on_register_updates: unsafe extern "C" fn(this: VoidPtr, registrar: VoidPtr),
}

/// Native virtual table layout for [`IGameSystem`], including lifecycle callbacks.
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

#[cfg(test)]
mod tests {
    use super::NativeGameInstance;

    #[test]
    fn native_game_instance_is_non_owning() {
        assert!(!std::mem::needs_drop::<NativeGameInstance>());
    }
}
