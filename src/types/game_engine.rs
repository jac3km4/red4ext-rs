use std::mem;

use super::{IScriptable, Native, Ref, ScriptClass, Type};
use crate::raw::root::RED4ext as red;
use crate::types::WeakRef;
use crate::{NativeRepr, VoidPtr};

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

    #[inline]
    pub fn get_system(&self, ty: &Type) -> Ref<ScriptableSystem> {
        let instance = unsafe { (self.vft().get_system)(ty) };
        if instance.is_null() {
            return Ref::default();
        }
        let instance: &red::IScriptable = unsafe { mem::transmute(&*instance) };
        let instance: &WeakRef<IScriptable> = unsafe { mem::transmute(&instance._base.ref_) };
        if let Some(instance) = instance.clone().upgrade() {
            return instance.cast().unwrap();
        }
        Ref::default()
    }

    #[inline]
    fn vft(&self) -> &RedGameInstanceVft {
        unsafe { &*self.0.instance.cast::<RedGameInstance>() }.vft()
    }
}

unsafe impl NativeRepr for GameInstance {
    const NAME: &'static str = "ScriptGameInstance";
}

#[derive(Default)]
#[repr(transparent)]
struct RedGameInstance(red::GameInstance);

impl RedGameInstance {
    #[inline]
    fn vft(&self) -> &RedGameInstanceVft {
        unsafe { mem::transmute(&*self.0.vtable_) }
    }
}

impl Drop for RedGameInstance {
    #[inline]
    fn drop(&mut self) {
        unsafe { (self.vft().destroy)(self) };
    }
}

#[repr(C)]
pub struct RedGameInstanceVft {
    destroy: unsafe fn(*mut RedGameInstance),
    get_system: unsafe extern "fastcall" fn(ty: &Type) -> *mut IScriptable,
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
        unsafe { mem::transmute(&*red::CGameEngine::Get()) }
    }

    fn game_instance(&self) -> &RedGameInstance {
        let s = unsafe { &*self.0.framework }.gameInstance;
        let s: &self::RedGameInstance = unsafe { mem::transmute(&*s) };
        s
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
