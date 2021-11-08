use std::ffi::{CStr, CString};
use std::time::Duration;

use crate::ffi::RED4ext;

#[ctor::ctor]
unsafe fn init() {
    std::thread::spawn(|| {
        // wait for game to start
        std::thread::sleep(Duration::from_secs(1));

        let class_name = CString::new("IScriptable").unwrap();
        let class_cname = RED4ext::CName::make_unique1(class_name.as_ptr());
        let result = CStr::from_ptr(RED4ext::CNamePool::Get(&class_cname)).to_bytes();
        std::fs::write("output.txt", result).unwrap();
    });
}
