use red4ext_rs::{
    Exportable, GlobalExport, Plugin, SemVer, U16CStr, export_plugin_symbols, exports, global,
    wcstr,
};

pub struct Example;

impl Plugin for Example {
    const AUTHOR: &'static U16CStr = wcstr!("me");
    const NAME: &'static U16CStr = wcstr!("example");
    const VERSION: SemVer = SemVer::new(0, 1, 0);

    fn exports() -> impl Exportable {
        exports![GlobalExport(global!(c"Add2", add2))]
    }
}

export_plugin_symbols!(Example);

fn add2(a: i32) -> i32 {
    a + 2
}
