/// shortcut for ResourcePath creation.
#[macro_export]
macro_rules! res_path {
    ($base:expr, /$lit:literal $($tt:tt)*) => {
        $crate::res_path!($base.join($lit), $($tt)*)
    };
    ($base:expr, ) => {
        $base
    };
    ($lit:literal $($tt:tt)*) => {
        $crate::res_path!($crate::interop::ResourcePath::builder().join($lit), $($tt)*).try_build()
    };
}

#[cfg(test)]
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
