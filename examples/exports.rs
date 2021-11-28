use red4ext_rs::prelude::*;

#[redscript_export]
fn sum_ints(ints: Vec<i32>) -> i32 {
    ints.iter().sum()
}

#[redscript_export]
fn to_lowercase(str: String) -> String {
    str.to_lowercase()
}

#[redscript_export]
fn concat_strings(strs: Vec<String>) -> String {
    strs.join("")
}

#[redscript_export]
fn on_menu_load(controller: Ref<RED4ext::IScriptable>) {
    call!("Max" (1i32, 2i32) -> i32);
    call!("RoundMath;Float" (1.2f32) -> i32);
    call!(controller, "Size" () -> i32);
}

#[ctor::ctor]
fn init() {
    rtti::on_register(register, post_register);
}

extern "C" fn register() {}

extern "C" fn post_register() {
    rtti::register_function("SumInts", sum_ints);
    rtti::register_function("ToLowercase", to_lowercase);
    rtti::register_function("ConcatStrings", concat_strings);
    rtti::register_function("OnMainMenuLoadTest", on_menu_load);
}
