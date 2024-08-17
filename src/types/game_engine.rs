use std::{mem, ptr};

use super::{IScriptable, RedArray, RedHashMap, Ref, Type};
use crate::class::{class_kind, ScriptClass};
use crate::raw::root::RED4ext as red;
use crate::types::WeakRef;
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

    #[inline]
    pub fn exists(&self, ty: &Type) -> bool {
        !unsafe { (self.vft().get_system)(self, ty) }.is_null()
    }

    #[inline]
    pub fn add_native_system<C: ScriptClass>(&mut self, ty: &mut Type, singleton: Ref<C>) {
        let (map, implementations, instances) = self.split_types();
        map.insert(ty as *mut _ as u32, singleton.clone().cast().unwrap());
        implementations.insert(ty as *mut _ as u32, ty);
        instances.push(singleton.cast().unwrap());
    }

    #[inline]
    #[allow(clippy::type_complexity)]
    fn split_types(
        &mut self,
    ) -> (
        &mut RedHashMap<u32, Ref<IScriptable>>,
        &mut RedHashMap<u32, &mut Type>,
        &mut RedArray<Ref<IScriptable>>,
    ) {
        unsafe {
            (
                &mut *(&mut self.0.systemMap as *mut _ as *mut RedHashMap<u32, Ref<IScriptable>>),
                &mut *(&mut self.0.systemImplementations as *mut _
                    as *mut RedHashMap<u32, &mut Type>),
                &mut *(&mut self.0.systemInstances as *mut _ as *mut RedArray<Ref<IScriptable>>),
            )
        }
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
    destroy: unsafe extern "fastcall" fn(this: *mut NativeGameInstance),
    get_system: unsafe extern "fastcall" fn(
        this: *const NativeGameInstance,
        ty: &Type,
    ) -> *mut red::IScriptable,
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

    pub fn get_mut<'a>() -> &'a mut Self {
        unsafe { &mut *(red::CGameEngine::Get() as *mut GameEngine) }
    }

    pub fn game_instance(&self) -> &NativeGameInstance {
        unsafe { &*((*self.0.framework).gameInstance as *const NativeGameInstance) }
    }

    pub fn game_instance_mut(&mut self) -> &mut NativeGameInstance {
        unsafe { &mut *((*self.0.framework).gameInstance as *mut NativeGameInstance) }
    }
}

#[derive(Debug)]
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

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct IGameSystem(red::game::IGameSystem);

impl IGameSystem {
    pub(crate) fn singleton() -> Ref<Self> {
        let job = JobHandle::new(10);
        let this = Ref::<Self>::new_with(|x| {
            x.0.gameInstance =
                Box::leak(Box::new(GameEngine::get().game_instance())) as *const _ as *mut _;
            unsafe {
                red::game::IGameSystem_OnInitialize(
                    Box::leak(Box::new(&mut x.0)) as *const _ as VoidPtr,
                    &job.0 as *const _,
                )
            };
        })
        .unwrap();
        mem::forget(job);
        this
    }
}

impl Drop for IGameSystem {
    fn drop(&mut self) {
        unsafe { red::game::IGameSystem_OnUninitialize(&mut self.0 as *const _ as VoidPtr) }
    }
}

unsafe impl ScriptClass for IGameSystem {
    type Kind = class_kind::Native;

    const NAME: &'static str = "gameIGameSystem";
}

impl Clone for IGameSystem {
    fn clone(&self) -> Self {
        unsafe { ptr::read(self) }
    }
}

impl AsRef<IScriptable> for IGameSystem {
    fn as_ref(&self) -> &IScriptable {
        unsafe { mem::transmute(&self.0._base._base) }
    }
}

#[derive(Default)]
#[repr(transparent)]
pub struct JobHandle(red::JobHandle);

impl JobHandle {
    pub fn new(timeout: usize) -> Self {
        Self(unsafe { red::JobHandle::new(timeout) })
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        unsafe { self.0.destruct() };
    }
}
