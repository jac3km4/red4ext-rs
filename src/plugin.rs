#[macro_export]
macro_rules! define_plugin {
    { name: $name:literal,
      author: $author:literal,
      version: $major:literal:$minor:literal:$patch:literal,
      on_register: $($on_register:tt)*
    } => {
        #[allow(non_snake_case)]
        #[no_mangle]
        unsafe extern "C" fn Main(handle: *const (), reason: $crate::RED4ext::EMainReason, sdk: *const $crate::RED4ext::Sdk) {
            match reason {
                $crate::RED4ext::EMainReason::Load =>
                    $crate::RED4extGlue::AddRTTICallback(Register as *mut _, PostRegister as *mut _, true),
                $crate::RED4ext::EMainReason::Unload => {}
            }
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        unsafe extern "C" fn Query(info: *mut $crate::RED4ext::PluginInfo) {
            $crate::RED4extGlue::DefinePlugin(
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
            $crate::RED4extGlue::GetSdkVersion()
        }

        #[allow(non_snake_case)]
        extern "C" fn Register() {}

        #[allow(non_snake_case)]
        extern "C" fn PostRegister() {
            $($on_register)*
        }
    };
}

#[macro_export]
macro_rules! define_trait_plugin {
    { name: $name:literal,
      author: $author:literal,
      plugin: $ty:ty
    } => {
        #[allow(non_snake_case)]
        #[no_mangle]
        unsafe extern "C" fn Main(handle: *const (), reason: $crate::RED4ext::EMainReason, sdk: *const $crate::RED4ext::Sdk) {
            match reason {
                $crate::RED4ext::EMainReason::Load =>
                    $crate::RED4extGlue::AddRTTICallback(Register as *mut _, PostRegister as *mut _, true),
                $crate::RED4ext::EMainReason::Unload => {
                    <$ty as $crate::plugin::Plugin>::unload();
                }
            }
        }

        #[allow(non_snake_case)]
        #[no_mangle]
        unsafe extern "C" fn Query(info: *mut $crate::RED4ext::PluginInfo) {
            let version = <$ty as $crate::plugin::Plugin>::version();
            $crate::RED4extGlue::DefinePlugin(
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
            $crate::RED4extGlue::GetSdkVersion()
        }

        #[allow(non_snake_case)]
        extern "C" fn Register() {
            <$ty as $crate::plugin::Plugin>::register();
        }

        #[allow(non_snake_case)]
        extern "C" fn PostRegister() {
            <$ty as $crate::plugin::Plugin>::post_register();
        }
    };
}

pub struct Version {
    pub major: u8,
    pub minor: u16,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u8, minor: u16, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

pub trait Plugin {
    fn version() -> Version;
    fn register() {}
    fn post_register() {}
    fn unload() {}
}
