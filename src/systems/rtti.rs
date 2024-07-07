use std::{mem, ptr};

use crate::raw::root::RED4ext as red;
use crate::types::{
    Bitfield, CName, Class, ClassHandle, Enum, Function, GlobalFunction, PoolRef, RedArray,
    RedHashMap, RwSpinLockReadGuard, RwSpinLockWriteGuard, Type,
};

#[repr(transparent)]
pub struct RttiSystem(red::CRTTISystem);

impl RttiSystem {
    #[inline]
    pub fn get<'a>() -> RwSpinLockReadGuard<'a, Self> {
        unsafe {
            let rtti = red::CRTTISystem_Get();
            let lock = &(*rtti).typesLock;
            RwSpinLockReadGuard::new(lock, ptr::NonNull::new_unchecked(rtti as _))
        }
    }

    /// Acquires a write lock on the RTTI system.
    ///
    /// # Notes
    /// You should avoid calling non-mut methods while the lock is held, because some methods
    /// may try to acquire the lock for reading, causing a deadlock.
    #[inline]
    pub fn get_mut<'a>() -> RwSpinLockWriteGuard<'a, Self> {
        unsafe {
            let rtti = red::CRTTISystem_Get();
            let lock = &(*rtti).typesLock;
            RwSpinLockWriteGuard::new(lock, ptr::NonNull::new_unchecked(rtti as _))
        }
    }

    #[inline]
    pub fn get_class(&self, name: CName) -> Option<&Class> {
        let ty = unsafe { (self.vft().get_class)(self, name) };
        unsafe { ty.cast::<Class>().as_ref() }
    }

    #[inline]
    pub fn get_class_mut(&mut self, name: CName) -> Option<&mut Class> {
        // implemented manually to avoid the game trying to obtain the type lock
        let (types, types_by_id, type_ids) = self.type_tables_mut();
        if let Some(ty) = types.get_mut(&name) {
            return ty.as_class_mut();
        }
        let &id = type_ids.get(&name)?;
        types_by_id.get_mut(&id)?.as_class_mut()
    }

    #[inline]
    pub fn get_type(&self, name: CName) -> Option<&Type> {
        let ty = unsafe { (self.vft().get_type)(self, name) };
        unsafe { ty.cast::<Type>().as_ref() }
    }

    #[inline]
    pub fn get_enum(&self, name: CName) -> Option<&Enum> {
        let ty = unsafe { (self.vft().get_enum)(self, name) };
        unsafe { ty.cast::<Enum>().as_ref() }
    }

    #[inline]
    pub fn get_bitfield(&self, name: CName) -> Option<&Bitfield> {
        let ty = unsafe { (self.vft().get_bitfield)(self, name) };
        unsafe { ty.cast::<Bitfield>().as_ref() }
    }

    #[inline]
    pub fn get_function(&self, name: CName) -> Option<&Function> {
        let ty = unsafe { (self.vft().get_function)(self, name) };
        unsafe { ty.cast::<Function>().as_ref() }
    }

    #[inline]
    pub fn get_native_types(&self) -> RedArray<&Type> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_native_types)(self, &mut out as *mut _ as *mut RedArray<*mut Type>)
        };
        out
    }

    #[inline]
    pub fn get_enums(&self) -> RedArray<&Enum> {
        let mut out = RedArray::default();
        unsafe { (self.vft().get_enums)(self, &mut out as *mut _ as *mut RedArray<*mut Enum>) };
        out
    }

    #[inline]
    pub fn get_bitfields(&self, scripted_only: bool) -> RedArray<&Bitfield> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_bitfields)(
                self,
                &mut out as *mut _ as *mut RedArray<*mut Bitfield>,
                scripted_only,
            )
        };
        out
    }

    #[inline]
    pub fn get_global_functions(&self) -> RedArray<&Function> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_global_functions)(
                self,
                &mut out as *mut _ as *mut RedArray<*mut Function>,
            )
        };
        out
    }

    #[inline]
    pub fn get_class_functions(&self) -> RedArray<&Function> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_class_functions)(
                self,
                &mut out as *mut _ as *mut RedArray<*mut Function>,
            )
        };
        out
    }

    /// retrieve base class and its inheritors, optionally including abstract classes.
    #[inline]
    pub fn get_classes(&self, base: &Class, include_abstract: bool) -> RedArray<&Class> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_classes)(
                self,
                base,
                &mut out as *mut _ as *mut RedArray<*mut Class>,
                None,
                include_abstract,
            )
        };
        out
    }

    /// retrieve derived classes, omitting base in the output.
    #[inline]
    pub fn get_derived_classes(&self, base: &Class) -> RedArray<&Class> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_derived_classes)(
                self,
                base,
                &mut out as *mut _ as *mut RedArray<*mut Class>,
            )
        };
        out
    }

    #[inline]
    pub fn get_class_by_script_name(&self, name: CName) -> Option<&Class> {
        let ty = unsafe { (self.vft().get_class_by_script_name)(self, name) };
        unsafe { ty.cast::<Class>().as_ref() }
    }

    #[inline]
    pub fn get_enum_by_script_name(&self, name: CName) -> Option<&Enum> {
        let ty = unsafe { (self.vft().get_enum_by_script_name)(self, name) };
        unsafe { ty.cast::<Enum>().as_ref() }
    }

    #[inline]
    pub fn register_function(&mut self, function: PoolRef<GlobalFunction>) {
        unsafe { (self.vft().register_function)(self, &*function) }
        // RTTI takes ownership of it from now on
        mem::forget(function);
    }

    #[inline]
    pub fn register_class(&mut self, mut class: ClassHandle) {
        // implemented manually to avoid the game trying to obtain the type lock
        let id = unsafe { red::RTTIRegistrator::GetNextId() };
        self.types_mut()
            .insert(class.as_ref().name(), class.as_mut().as_type_mut());
        self.types_by_id_mut()
            .insert(id, class.as_mut().as_type_mut());
        self.type_ids_mut().insert(class.as_ref().name(), id);
    }

    #[inline]
    fn vft(&self) -> &RttiSystemVft {
        unsafe { &*(self.0._base.vtable_ as *const RttiSystemVft) }
    }

    #[inline]
    fn types_mut(&mut self) -> &mut RedHashMap<CName, &mut Type> {
        unsafe { &mut *(&mut self.0.types as *mut _ as *mut RedHashMap<CName, &mut Type>) }
    }

    #[inline]
    fn types_by_id_mut(&mut self) -> &mut RedHashMap<u32, &mut Type> {
        unsafe { &mut *(&mut self.0.typesByAsyncId as *mut _ as *mut RedHashMap<u32, &mut Type>) }
    }

    #[inline]
    fn type_ids_mut(&mut self) -> &mut RedHashMap<CName, u32> {
        unsafe { &mut *(&mut self.0.typeAsyncIds as *mut _ as *mut RedHashMap<CName, u32>) }
    }

    #[inline]
    #[allow(clippy::type_complexity)]
    fn type_tables_mut(
        &mut self,
    ) -> (
        &mut RedHashMap<CName, &mut Type>,
        &mut RedHashMap<u32, &mut Type>,
        &mut RedHashMap<CName, u32>,
    ) {
        unsafe {
            (
                &mut *(&mut self.0.types as *mut _ as *mut RedHashMap<CName, &mut Type>),
                &mut *(&mut self.0.typesByAsyncId as *mut _ as *mut RedHashMap<u32, &mut Type>),
                &mut *(&mut self.0.typeAsyncIds as *mut _ as *mut RedHashMap<CName, u32>),
            )
        }
    }
}

#[repr(C)]
struct RttiSystemVft {
    get_type: unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *mut Type,
    get_type_by_async_id:
        unsafe extern "fastcall" fn(this: *const RttiSystem, async_id: u32) -> *mut Type,
    get_class: unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *mut Class,
    get_enum: unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *mut Enum,
    get_bitfield:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *mut Bitfield,
    _sub_28: unsafe extern "fastcall" fn(this: *const RttiSystem),
    get_function:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *mut Function,
    _sub_38: unsafe extern "fastcall" fn(this: *const RttiSystem),
    get_native_types:
        unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut RedArray<*mut Type>),
    get_global_functions:
        unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut RedArray<*mut Function>),
    _sub_50: unsafe extern "fastcall" fn(this: *const RttiSystem),
    get_class_functions:
        unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut RedArray<*mut Function>),
    get_enums: unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut RedArray<*mut Enum>),
    get_bitfields: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        out: *mut RedArray<*mut Bitfield>,
        scripted_only: bool,
    ),
    get_classes: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        base_class: *const Class,
        out: *mut RedArray<*mut Class>,
        filter: Option<unsafe extern "C" fn(*const Class) -> bool>,
        include_abstract: bool,
    ),
    get_derived_classes: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        base_class: *const Class,
        out: *mut RedArray<*mut Class>,
    ),
    register_type: unsafe extern "fastcall" fn(this: *mut RttiSystem, ty: *mut Type, async_id: u32),
    _sub_88: unsafe extern "fastcall" fn(this: *const RttiSystem),
    _sub_90: unsafe extern "fastcall" fn(this: *const RttiSystem),
    unregister_type: unsafe extern "fastcall" fn(this: *mut RttiSystem, ty: *mut Type),
    register_function:
        unsafe extern "fastcall" fn(this: *const RttiSystem, function: *const GlobalFunction),
    unregister_function:
        unsafe extern "fastcall" fn(this: *const RttiSystem, function: *const GlobalFunction),
    _sub_b0: unsafe extern "fastcall" fn(this: *const RttiSystem),
    _sub_b8: unsafe extern "fastcall" fn(this: *const RttiSystem),
    // FIXME: crashes when used, signature is probably wrong
    _add_register_callback: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        function: unsafe extern "C" fn() -> (),
    ),
    // FIXME: crashes when used, signature is probably wrong
    _add_post_register_callback: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        function: unsafe extern "C" fn() -> (),
    ),
    _sub_d0: unsafe extern "fastcall" fn(this: *const RttiSystem),
    _sub_d8: unsafe extern "fastcall" fn(this: *const RttiSystem),
    _create_scripted_class: unsafe extern "fastcall" fn(
        this: *mut RttiSystem,
        name: CName,
        flags: red::CClass_Flags,
        parent: *const Class,
    ),
    // FIXME: signature is wrong, but how to represent name and value of enumerator ?
    // https://github.com/WopsS/RED4ext.SDK/blob/124984353556f7b343041b810040062fbaa96196/include/RED4ext/RTTISystem.hpp#L50
    _create_scripted_enum: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        name: CName,
        size: i8,
        variants: *mut RedArray<u64>,
    ),
    // FIXME: signature is wrong, but how to represent name and bit ?
    // https://github.com/WopsS/RED4ext.SDK/blob/124984353556f7b343041b810040062fbaa96196/include/RED4ext/RTTISystem.hpp#L54
    _create_scripted_bitfield:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName, bits: *mut RedArray<u64>),
    _initialize_script_runtime: unsafe extern "fastcall" fn(this: *const RttiSystem),
    register_script_name: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        native_name: CName,
        script_name: CName,
    ),
    get_class_by_script_name:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Class,
    get_enum_by_script_name:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Enum,
    // FIXME: crashes when used, signature is probably wrong
    _convert_native_to_script_name:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: red::CName) -> red::CName,
    // FIXME: crashes when used, signature is probably wrong
    _convert_script_to_native_name:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: red::CName) -> red::CName,
}

#[derive(Debug)]
pub struct RttiRegistrator;

impl RttiRegistrator {
    pub fn add(
        register: Option<unsafe extern "C" fn()>,
        post_register: Option<unsafe extern "C" fn()>,
    ) {
        unsafe { red::RTTIRegistrator::Add(register, post_register, false) };
    }
}
