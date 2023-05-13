pub mod error;
pub mod interop;

#[allow(clippy::missing_safety_doc)]
#[cxx::bridge]
pub mod ffi {
    // define extern enums here
    #[repr(u8)]
    pub enum EMainReason {
        Load = 0,
        Unload = 1,
    }

    #[namespace = "RED4ext"]
    unsafe extern "C++" {
        include!("RED4ext/RED4ext.hpp");

        #[namespace = "RED4ext::Memory"]
        type IAllocator;

        type IScriptable;
        type CClass;
        type CBaseFunction;
        type CGlobalFunction;
        type CClassFunction;
        #[cxx_name = "IRTTISystem"]
        type IRttiSystem;
        #[cxx_name = "CBaseRTTIType"]
        type CBaseRttiType;
        type CStackFrame;
        type CProperty;
        type PluginInfo;
        type Sdk;
        type EMainReason;

        type CName = crate::interop::CName;
        type CString = crate::interop::RedString;
        #[cxx_name = "TweakDBID"]
        type TweakDbId = crate::interop::TweakDbId;
        #[cxx_name = "ItemID"]
        type ItemId = crate::interop::ItemId;
        #[cxx_name = "EntityID"]
        type EntityId = crate::interop::EntityId;
        type CStackType = crate::interop::StackArg;
        type Variant = crate::interop::Variant;

        #[cxx_name = "GetFunction"]
        fn get_function(self: Pin<&mut IRttiSystem>, name: CName) -> *mut CGlobalFunction;

        #[cxx_name = "GetClass"]
        fn get_class(self: Pin<&mut IRttiSystem>, name: CName) -> *mut CClass;

        #[cxx_name = "GetType"]
        fn get_type(self: Pin<&mut IRttiSystem>, name: CName) -> *mut CBaseRttiType;

        #[cxx_name = "RegisterFunction"]
        unsafe fn register_function(self: Pin<&mut IRttiSystem>, func: *mut CGlobalFunction);

        #[cxx_name = "GetType"]
        fn get_class(self: Pin<&mut IScriptable>) -> *mut CClass;

        #[cxx_name = "GetName"]
        fn get_name(self: &CBaseRttiType) -> CName;

        #[cxx_name = "GetParameter"]
        unsafe fn get_parameter(frame: *mut CStackFrame, mem: VoidPtr);

        #[cxx_name = "Step"]
        fn step(self: Pin<&mut CStackFrame>);

        #[cxx_name = "GetType"]
        fn get_type(self: &Variant) -> *mut CBaseRttiType;

        #[cxx_name = "GetDataPtr"]
        fn get_data_ptr(self: &Variant) -> VoidPtr;

        #[cxx_name = "Fill"]
        unsafe fn fill(self: Pin<&mut Variant>, typ: *const CBaseRttiType, data: VoidPtr) -> bool;
    }

    #[namespace = "glue"]
    unsafe extern "C++" {
        include!("glue.hpp");

        type VoidPtr = super::interop::VoidPtr;

        #[cxx_name = "CreateNativeFunction"]
        fn new_native_function(
            name: &str,
            short_name: &str,
            mem: VoidPtr,
            args: &[CName],
            ret: CName,
            errors: &mut Vec<usize>,
        ) -> *mut CGlobalFunction;

        #[cxx_name = "GetRTTI"]
        fn get_rtti() -> *mut IRttiSystem;

        #[cxx_name = "AddRTTICallback"]
        fn add_rtti_callback(reg_func: VoidPtr, post_reg_func: VoidPtr, unused: bool);

        #[cxx_name = "ConstructStringAt"]
        unsafe fn construct_string_at(str: *mut CString, text: &str, alloc: *mut IAllocator);

        #[cxx_name = "Execute"]
        unsafe fn execute_function(
            instance: VoidPtr,
            func: Pin<&mut CBaseFunction>,
            mem: VoidPtr,
            args: &[CStackType],
        ) -> bool;

        #[cxx_name = "DefinePlugin"]
        unsafe fn define_plugin(
            info: *mut PluginInfo,
            name: *const u16,
            author: *const u16,
            major: u8,
            minor: u16,
            patch: u32,
        );

        #[cxx_name = "GetSdkVersion"]
        fn get_sdk_version() -> u32;

        #[cxx_name = "AllocArray"]
        fn alloc_array(arr: VoidPtr, cap: u32, elem_size: u32);

        #[cxx_name = "GetParameters"]
        fn get_parameters(func: &CBaseFunction) -> &[*const CProperty];

        #[cxx_name = "GetReturn"]
        fn get_return(func: &CBaseFunction) -> *const CProperty;

        #[cxx_name = "GetPropertyType"]
        unsafe fn get_property_type(prop: *const CProperty) -> *const CBaseRttiType;

        #[cxx_name = "ResolveCName"]
        fn resolve_cname(cname: &CName) -> &'static str;

        #[cxx_name = "GetMethod"]
        fn get_method(cls: &CClass, name: &CName) -> *mut CClassFunction;
    }
}
