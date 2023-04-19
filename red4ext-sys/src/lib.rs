#![allow(clippy::missing_safety_doc)]

pub mod interop;

#[cxx::bridge]
pub mod ffi {
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
        type PluginInfo;
        type Sdk;

        type EMainReason = crate::interop::MainReason;
        type CName = crate::interop::CName;
        type CString = crate::interop::REDString;
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

        #[cxx_name = "GetFunction"]
        fn get_function(self: &CClass, name: CName) -> *mut CClassFunction;

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
            func: *mut CBaseFunction,
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
    }
}
