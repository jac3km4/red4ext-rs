use red4ext_rs::prelude::*;

#[redscript]
fn to_lowercase(a: String) -> String {
    a.to_lowercase()
}

#[ctor::ctor]
fn init() {
    on_register(register, post_register);
}

extern "C" fn register() {}

extern "C" fn post_register() {
    register_function!("ToLowercase", to_lowercase);
}
