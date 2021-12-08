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
