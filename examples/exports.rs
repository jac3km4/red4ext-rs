use red4ext_rs::prelude::*;

#[redscript]
fn sum_ints(ints: Vec<i32>) -> i32 {
    ints.iter().sum()
}

#[redscript]
fn to_lowercase(str: String) -> String {
    str.to_lowercase()
}

#[redscript]
fn concat_strings(strs: Vec<String>) -> String {
    strs.join("")
}

#[ctor::ctor]
fn init() {
    on_register(register, post_register);
}

extern "C" fn register() {}

extern "C" fn post_register() {
    register_function!("SumInts", sum_ints);
    register_function!("ToLowercase", to_lowercase);
    register_function!("ConcatStrings", concat_strings);
}
