use crate::raw::root::RED4ext as red;
use crate::types::{Array, Bitfield, CName, Class, Enum, Function, GlobalFunction, Type};

#[repr(transparent)]
pub struct RttiSystem(red::CRTTISystem);

impl RttiSystem {
    pub fn get() -> &'static Self {
        unsafe { &*(red::CRTTISystem_Get() as *const RttiSystem) }
    }

    #[inline]
    pub fn get_class(&self, name: CName) -> Option<&Class> {
        let ty = unsafe { (self.vft().get_class)(self, name) };
        unsafe { ty.cast::<Class>().as_ref() }
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
    pub fn get_native_types(&self) -> Array<&Type> {
        let mut out = Array::default();
        unsafe {
            (self.vft().get_native_types)(self, &mut out as *mut _ as *mut Array<*const Type>)
        };
        out
    }

    #[inline]
    pub fn get_enums(&self) -> Array<&Enum> {
        let mut out = Array::default();
        unsafe { (self.vft().get_enums)(self, &mut out as *mut _ as *mut Array<*const Enum>) };
        out
    }

    #[inline]
    pub fn get_bitfields(&self, scripted_only: bool) -> Array<&Bitfield> {
        let mut out = Array::default();
        unsafe {
            (self.vft().get_bitfields)(
                self,
                &mut out as *mut _ as *mut Array<*const Bitfield>,
                scripted_only,
            )
        };
        out
    }

    #[inline]
    pub fn get_global_functions(&self) -> Array<&Function> {
        let mut out = Array::default();
        unsafe {
            (self.vft().get_global_functions)(
                self,
                &mut out as *mut _ as *mut Array<*const Function>,
            )
        };
        out
    }

    #[inline]
    pub fn get_class_functions(&self) -> Array<&Function> {
        let mut out = Array::default();
        unsafe {
            (self.vft().get_class_functions)(
                self,
                &mut out as *mut _ as *mut Array<*const Function>,
            )
        };
        out
    }

    /// retrieve base class and its inheritors, optionally including abstract classes.
    #[inline]
    pub fn get_classes(&self, base: &Class, include_abstract: bool) -> Array<&Class> {
        let mut out = Array::default();
        unsafe {
            (self.vft().get_classes)(
                self,
                base,
                &mut out as *mut _ as *mut Array<*const Class>,
                None,
                include_abstract,
            )
        };
        out
    }

    /// retrieve derived classes, omitting base in the output.
    #[inline]
    pub fn get_derived_classes(&self, base: &Class) -> Array<&Class> {
        let mut out = Array::default();
        unsafe {
            (self.vft().get_derived_classes)(
                self,
                base,
                &mut out as *mut _ as *mut Array<*const Class>,
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
    pub fn register_function(&self, function: &GlobalFunction) {
        unsafe { (self.vft().register_function)(self, function) }
    }

    #[inline]
    fn vft(&self) -> &RttiSystemVft {
        unsafe { &*(self.0._base.vtable_ as *const RttiSystemVft) }
    }
}

#[repr(C)]
struct RttiSystemVft {
    get_type: unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Type,
    get_type_by_async_id:
        unsafe extern "fastcall" fn(this: *const RttiSystem, async_id: u32) -> *const Type,
    get_class: unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Class,
    get_enum: unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Enum,
    get_bitfield:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Bitfield,
    _sub_28: unsafe extern "fastcall" fn(this: *const RttiSystem),
    get_function:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName) -> *const Function,
    _sub_38: unsafe extern "fastcall" fn(this: *const RttiSystem),
    get_native_types:
        unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut Array<*const Type>),
    get_global_functions:
        unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut Array<*const Function>),
    _sub_50: unsafe extern "fastcall" fn(this: *const RttiSystem),
    get_class_functions:
        unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut Array<*const Function>),
    get_enums: unsafe extern "fastcall" fn(this: *const RttiSystem, out: *mut Array<*const Enum>),
    get_bitfields: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        out: *mut Array<*const Bitfield>,
        scripted_only: bool,
    ),
    get_classes: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        base_class: *const Class,
        out: *mut Array<*const Class>,
        filter: Option<unsafe extern "C" fn(*const Class) -> bool>,
        include_abstract: bool,
    ),
    get_derived_classes: unsafe extern "fastcall" fn(
        this: *const RttiSystem,
        base_class: *const Class,
        out: *mut Array<*const Class>,
    ),
    register_type:
        unsafe extern "fastcall" fn(this: *const RttiSystem, ty: *const Type, async_id: u32),
    _sub_88: unsafe extern "fastcall" fn(this: *const RttiSystem),
    _sub_90: unsafe extern "fastcall" fn(this: *const RttiSystem),
    unregister_type: unsafe extern "fastcall" fn(this: *const RttiSystem, ty: *const Type),
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
        this: *const RttiSystem,
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
        variants: *mut Array<u64>,
    ),
    // FIXME: signature is wrong, but how to represent name and bit ?
    // https://github.com/WopsS/RED4ext.SDK/blob/124984353556f7b343041b810040062fbaa96196/include/RED4ext/RTTISystem.hpp#L54
    _create_scripted_bitfield:
        unsafe extern "fastcall" fn(this: *const RttiSystem, name: CName, bits: *mut Array<u64>),
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
