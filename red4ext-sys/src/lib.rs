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

    /// see [gameEItemIDFlag](https://nativedb.red4ext.com/gameEItemIDFlag)
    /// and [CET initialization](https://github.com/maximegmd/CyberEngineTweaks/blob/v1.24.1/src/scripting/Scripting.cpp#L311).
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq)]
    #[repr(u8)]
    #[allow(non_camel_case_types)]
    pub enum gameEItemIDFlag {
        None = 0,
        Preview = 1,
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq)]
    #[repr(u8)]
    #[allow(non_camel_case_types)]
    pub enum gamedataItemStructure {
        BlueprintStackable = 0,
        Stackable = 1,
        Unique = 2,
        Count = 3,
        Invalid = 4,
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
        type IRTTISystem;
        type CRTTISystem;
        type CBaseRTTIType;
        type CStackFrame;
        type CProperty;
        type PluginInfo;
        type Sdk;
        type EMainReason;

        type CName = crate::interop::CName;
        type CString = crate::interop::REDString;
        type TweakDBID = crate::interop::TweakDBID;
        type ItemID = crate::interop::ItemID;
        type CStackType = crate::interop::StackArg;
        type Variant = crate::interop::Variant;

        #[cxx_name = "GetFunction"]
        fn get_function(self: Pin<&mut IRTTISystem>, name: CName) -> *mut CGlobalFunction;

        #[cxx_name = "GetClass"]
        fn get_class(self: Pin<&mut IRTTISystem>, name: CName) -> *mut CClass;

        #[cxx_name = "GetType"]
        fn get_type(self: Pin<&mut IRTTISystem>, name: CName) -> *mut CBaseRTTIType;

        #[cxx_name = "RegisterFunction"]
        unsafe fn register_function(self: Pin<&mut IRTTISystem>, func: *mut CGlobalFunction);

        #[cxx_name = "GetType"]
        fn get_class(self: Pin<&mut IScriptable>) -> *mut CClass;

        #[cxx_name = "GetName"]
        fn get_name(self: &CBaseRTTIType) -> CName;

        #[cxx_name = "GetParameter"]
        unsafe fn get_parameter(frame: *mut CStackFrame, mem: VoidPtr);

        #[cxx_name = "Step"]
        fn step(self: Pin<&mut CStackFrame>);

        #[cxx_name = "GetType"]
        fn get_type(self: &Variant) -> *mut CBaseRTTIType;

        #[cxx_name = "GetDataPtr"]
        fn get_data_ptr(self: &Variant) -> VoidPtr;

        #[cxx_name = "Fill"]
        unsafe fn fill(self: Pin<&mut Variant>, typ: *const CBaseRTTIType, data: VoidPtr) -> bool;
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
        ) -> *mut CGlobalFunction;

        #[cxx_name = "GetRTTI"]
        fn get_rtti() -> *mut IRTTISystem;

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
        unsafe fn get_property_type(prop: *const CProperty) -> *const CBaseRTTIType;

        #[cxx_name = "ResolveCName"]
        fn resolve_cname(cname: &CName) -> &'static str;

        #[cxx_name = "GetMethod"]
        fn get_method(cls: &CClass, name: &CName) -> *mut CClassFunction;
    }
}
