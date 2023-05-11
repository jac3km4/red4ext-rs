#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod conv;
pub mod error;
#[doc(hidden)]
pub mod invocable;
#[doc(hidden)]
pub mod logger;
pub mod plugin;
pub mod prelude;
#[doc(hidden)]
pub mod rtti;
pub mod types;

#[cfg(feature = "macros")]
pub use red4ext_macros as macros;
#[doc(hidden)]
pub use red4ext_sys::ffi;
pub use wchar;

/// shortcut for ResourcePath creation.
#[cfg(feature = "macros")]
#[macro_export]
macro_rules! res_path {
    ($base:expr, /$lit:literal $($tt:tt)*) => {
        $crate::res_path!($base.join($lit), $($tt)*)
    };
    ($base:expr, ) => {
        $base
    };
    ($lit:literal $($tt:tt)*) => {
        $crate::prelude::ResourcePath::new($crate::res_path!(std::path::PathBuf::from($lit), $($tt)*)
         .to_str()
         .unwrap())
    };
}

#[cfg(all(test, feature = "macros"))]
mod tests {
    #[test]
    fn res_path() {
        use crate::res_path;
        assert!(res_path!("").is_err());
        assert!(res_path!(".." / "somewhere" / "in" / "archive" / "custom.ent").is_err());
        assert!(res_path!("base" / "somewhere" / "in" / "archive" / "custom.ent").is_ok());
        assert!(res_path!("custom.ent").is_ok());
        assert!(res_path!(".custom.ent").is_ok());
    }
}
