pub use red4ext_sys::interop::EMainReason as MainReason;

use crate::logger::SdkLogger;

pub type PluginHandle = usize;

pub struct SemVer;
pub struct Hooking;
pub struct GameStates;

#[repr(C)]
pub struct Sdk {
    pub(crate) runtime: &'static SemVer,
    pub(crate) logger: &'static SdkLogger,
    pub(crate) hooking: &'static Hooking,
    pub(crate) game_states: &'static GameStates,
}

#[macro_export]
macro_rules! define_plugin {
    { name: $name:literal,
      author: $author:literal,
      version: $major:literal:$minor:literal:$patch:literal,
      on_register: $($on_register:tt)*
    } => {
        mod __raw_api {
            use super::*;

            #[allow(non_snake_case)]
            #[no_mangle]
            unsafe extern "C" fn Main(handle: $crate::plugin::PluginHandle, reason: $crate::plugin::MainReason, sdk: &'static $crate::plugin::Sdk) {
                match reason {
                    $crate::plugin::MainReason::Load => {
                        $crate::logger::Logger::init(sdk, handle).ok();

                        $crate::ffi::add_rtti_callback($crate::types::VoidPtr(Register as *mut _), $crate::types::VoidPtr(PostRegister as *mut _), true)
                    }
                    _ => {}
                }
            }

            #[allow(non_snake_case)]
            #[no_mangle]
            unsafe extern "C" fn Query(info: *mut $crate::ffi::PluginInfo) {
                $crate::ffi::define_plugin(
                    info as _,
                    $crate::wchar::wchz!($name).as_ptr(),
                    $crate::wchar::wchz!($author).as_ptr(),
                    $major,
                    $minor,
                    $patch,
                );
            }

            #[allow(non_snake_case)]
            #[no_mangle]
            extern "C" fn Supports() -> u32 {
                $crate::ffi::get_sdk_version()
            }

            #[allow(non_snake_case)]
            extern "C" fn Register() {}

            #[allow(non_snake_case)]
            extern "C" fn PostRegister() {
                $($on_register)*
            }
        }
    };
}

#[macro_export]
macro_rules! define_trait_plugin {
    { name: $name:literal,
      author: $author:literal,
      plugin: $ty:ty
    } => {
        mod __raw_api {
            use super::*;

            #[allow(non_snake_case)]
            #[no_mangle]
            unsafe extern "C" fn Main(handle: $crate::plugin::PluginHandle, reason: $crate::plugin::MainReason, sdk: &'static $crate::plugin::Sdk) {
                match reason {
                    $crate::plugin::MainReason::Load => {
                        $crate::logger::Logger::init(sdk, handle).ok();

                        $crate::ffi::add_rtti_callback($crate::VoidPtr(Register as *mut _), $crate::VoidPtr(PostRegister as *mut _), true)
                    }
                    $crate::plugin::MainReason::Unload => {
                        <$ty as $crate::plugin::Plugin>::unload();
                    }
                    _ => {}
                }
            }

            #[allow(non_snake_case)]
            #[no_mangle]
            unsafe extern "C" fn Query(info: *mut $crate::ffi::PluginInfo) {
                let version = <$ty as $crate::plugin::Plugin>::VERSION;
                $crate::ffi::define_plugin(
                    info as _,
                    $crate::wchar::wchz!($name).as_ptr(),
                    $crate::wchar::wchz!($author).as_ptr(),
                    version.major,
                    version.minor,
                    version.patch,
                );
            }

            #[allow(non_snake_case)]
            #[no_mangle]
            extern "C" fn Supports() -> u32 {
                $crate::ffi::get_sdk_version()
            }

            #[allow(non_snake_case)]
            extern "C" fn Register() {
                <$ty as $crate::plugin::Plugin>::register();
            }

            #[allow(non_snake_case)]
            extern "C" fn PostRegister() {
                <$ty as $crate::plugin::Plugin>::post_register();
            }
        }
    };
}

pub struct Version {
    pub major: u8,
    pub minor: u16,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u8, minor: u16, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

pub trait Plugin {
    const VERSION: Version;

    fn register() {}
    fn post_register() {}
    fn unload() {}
}
