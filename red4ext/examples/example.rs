use red4ext_rs::prelude::*;

define_plugin! {
    name: "example",
    author: "jekky",
    version: 1:0:0,
    on_register: {
        register_function!("SumInts", sum_ints);
        register_function!("CallDemo", call_demo);
    }
}

fn sum_ints(ints: Vec<i32>) -> i32 {
    ints.iter().sum()
}

fn call_demo() {
    let res = call!("OperatorAdd;Uint32Uint32;Uint32" (2u32, 2u32) -> u32);
    info!("2 + 2 = {}", res);
}
