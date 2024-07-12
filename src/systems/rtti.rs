use std::{mem, ptr};

use crate::raw::root::RED4ext as red;
use crate::types::{
    Bitfield, CName, Class, ClassFlags, ClassHandle, Enum, Function, GlobalFunction, PoolRef,
    RedArray, RedHashMap, RwSpinLockReadGuard, RwSpinLockWriteGuard, Type,
};

/// The RTTI system containing information about all types in the game.
///
/// # Example
/// ```rust
/// use red4rs::types::CName;
/// use red4rs::RttiSystem;
///
/// fn rtti_example() {
///     let rtti = RttiSystem::get();
///     let class = rtti.get_class(CName::new("IScriptable")).unwrap();
///     for method in class.methods() {
///         // do something with the method
///     }
/// }
/// ```
#[repr(transparent)]
pub struct RttiSystem(red::CRTTISystem);

impl RttiSystem {
    /// Acquire a read lock on the RTTI system.
    #[inline]
    pub fn get<'a>() -> RwSpinLockReadGuard<'a, Self> {
        unsafe {
            let rtti = red::CRTTISystem_Get();
            let lock = &(*rtti).typesLock;
            RwSpinLockReadGuard::new(lock, ptr::NonNull::new_unchecked(rtti as _))
        }
    }

    /// Retrieve a class by its name.
    #[inline]
    pub fn get_class(&self, name: CName) -> Option<&Class> {
        let ty = unsafe { (self.vft().get_class)(self, name) };
        unsafe { ty.cast::<Class>().as_ref() }
    }

    /// Retrieve a type by its name.
    #[inline]
    pub fn get_type(&self, name: CName) -> Option<&Type> {
        let ty = unsafe { (self.vft().get_type)(self, name) };
        unsafe { ty.cast::<Type>().as_ref() }
    }

    /// Retrieve an enum by its name.
    #[inline]
    pub fn get_enum(&self, name: CName) -> Option<&Enum> {
        let ty = unsafe { (self.vft().get_enum)(self, name) };
        unsafe { ty.cast::<Enum>().as_ref() }
    }

    /// Retrieve a bitfield by its name.
    #[inline]
    pub fn get_bitfield(&self, name: CName) -> Option<&Bitfield> {
        let ty = unsafe { (self.vft().get_bitfield)(self, name) };
        unsafe { ty.cast::<Bitfield>().as_ref() }
    }

    /// Retrieve a function by its name.
    #[inline]
    pub fn get_function(&self, name: CName) -> Option<&Function> {
        let ty = unsafe { (self.vft().get_function)(self, name) };
        unsafe { ty.cast::<Function>().as_ref() }
    }

    /// Retrieve all native types and collect them into a [`RedArray`]`.
    #[inline]
    pub fn get_native_types(&self) -> RedArray<&Type> {
        let mut out = RedArray::default();
        unsafe {
            (self.vft().get_native_types)(self, &mut out as *mut _ as *mut RedArray<*mut Type>)
        };
        out
    }

    /// Retrieve all enums and collect them into a [`RedArray`]`.
    #[inline]
    pub fn get_enums(&self) -> RedArray<&Enum> {
        let mut out = RedArray::default();
        unsafe { (self.vft().get_enums)(self, &mut out as *mut _ as *mut RedArray<*mut Enum>) };
        out
    }

    /// Retrieve all bitfields and collect them into a [`RedArray`]`.
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

    /// Retrieve all global functions and collect them into a [`RedArray`]`.
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

    /// Retrieve all instance methods and collect them into a [`RedArray`]`.
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

    /// Retrieve base class and its inheritors, optionally including abstract classes.
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

    /// Retrieve derived classes, omitting base in the output.
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

    /// Retrieve a class by its script name.
    #[inline]
    pub fn get_class_by_script_name(&self, name: CName) -> Option<&Class> {
        let ty = unsafe { (self.vft().get_class_by_script_name)(self, name) };
        unsafe { ty.cast::<Class>().as_ref() }
    }

    /// Retrieve an enum by its script name.
    #[inline]
    pub fn get_enum_by_script_name(&self, name: CName) -> Option<&Enum> {
        let ty = unsafe { (self.vft().get_enum_by_script_name)(self, name) };
        unsafe { ty.cast::<Enum>().as_ref() }
    }

    /// Retrieve a reference to a map of all types by name.
    #[inline]
    pub fn types(&self) -> &RedHashMap<CName, &Type> {
        unsafe { &*(&self.0.types as *const _ as *const RedHashMap<CName, &Type>) }
    }

    /// Retrieve a reference to a map of all script to native name aliases.
    #[inline]
    pub fn script_to_native_map(&self) -> &RedHashMap<CName, CName> {
        unsafe { &*(&self.0.scriptToNative as *const _ as *const RedHashMap<CName, CName>) }
    }

    /// Retrieve a reference to a map of all native to script name aliases.
    #[inline]
    pub fn native_to_script_map(&self) -> &RedHashMap<CName, CName> {
        unsafe { &*(&self.0.nativeToScript as *const _ as *const RedHashMap<CName, CName>) }
    }

    #[inline]
    fn vft(&self) -> &RttiSystemVft {
        unsafe { &*(self.0._base.vtable_ as *const RttiSystemVft) }
    }
}

/// The RTTI system containing information about all types in the game.
/// This variant allows for modifying the RTTI system and locks it for exclusive access.
#[repr(transparent)]
pub struct RttiSystemMut(red::CRTTISystem);

impl RttiSystemMut {
    /// Acquire a write lock on the RTTI system. You should be careful not to hold the lock for
    /// too long, because interleaving reads and write operations can lead to deadlocks.
    #[inline]
    pub fn get() -> RwSpinLockWriteGuard<'static, Self> {
        unsafe {
            let rtti = red::CRTTISystem_Get();
            let lock = &(*rtti).typesLock;
            RwSpinLockWriteGuard::new(lock, ptr::NonNull::new_unchecked(rtti as _))
        }
    }

    /// Retrieve a mutable reference to a class by its name
    pub fn get_class(&mut self, name: CName) -> Option<&mut Class> {
        // implemented manually to avoid the game trying to obtain the type lock
        let (types, types_by_id, type_ids) = self.split_types();
        if let Some(ty) = types.get_mut(&name) {
            return ty.as_class_mut();
        }
        let &id = type_ids.get(&name)?;
        types_by_id.get_mut(&id)?.as_class_mut()
    }

    /// Register a new [`ClassHandle`] with the RTTI system.
    /// The handle can be obtained from
    /// [`NativeClass::new_handle`](crate::types::NativeClass::new_handle).
    pub fn register_class(&mut self, mut class: ClassHandle) {
        // implemented manually to avoid the game trying to obtain the type lock
        let id = unsafe { red::RTTIRegistrator::GetNextId() };
        self.types()
            .insert(class.as_ref().name(), class.as_mut().as_type_mut());
        self.types_by_id().insert(id, class.as_mut().as_type_mut());
        self.type_ids().insert(class.as_ref().name(), id);
    }

    /// Register a new [`GlobalFunction`] with the RTTI system.
    /// The function can be obtained from [`GlobalFunction::new`].
    #[inline]
    pub fn register_function(&mut self, function: PoolRef<GlobalFunction>) {
        unsafe { (self.vft().register_function)(self, &*function) }
        // RTTI takes ownership of it from now on
        mem::forget(function);
    }

    #[inline]
    fn types(&mut self) -> &mut RedHashMap<CName, &mut Type> {
        unsafe { &mut *(&mut self.0.types as *mut _ as *mut RedHashMap<CName, &mut Type>) }
    }

    #[inline]
    fn types_by_id(&mut self) -> &mut RedHashMap<u32, &mut Type> {
        unsafe { &mut *(&mut self.0.typesByAsyncId as *mut _ as *mut RedHashMap<u32, &mut Type>) }
    }

    #[inline]
    fn type_ids(&mut self) -> &mut RedHashMap<CName, u32> {
        unsafe { &mut *(&mut self.0.typeAsyncIds as *mut _ as *mut RedHashMap<CName, u32>) }
    }

    #[inline]
    #[allow(clippy::type_complexity)]
    fn split_types(
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

    #[inline]
    fn vft(&self) -> &RttiSystemVft {
        unsafe { &*(self.0._base.vtable_ as *const RttiSystemVft) }
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
        unsafe extern "fastcall" fn(this: *const RttiSystemMut, function: *const GlobalFunction),
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
        flags: ClassFlags,
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

/// A helper struct to set up RTTI registration callbacks.
#[derive(Debug)]
pub struct RttiRegistrator;

impl RttiRegistrator {
    /// Add a new RTTI registration callback.
    pub fn add(
        register: Option<unsafe extern "C" fn()>,
        post_register: Option<unsafe extern "C" fn()>,
    ) {
        unsafe { red::RTTIRegistrator::Add(register, post_register, false) };
    }
}
