use std::time::Duration;

use crate::prelude::*;

#[redscript]
fn testing(a: i32) -> i32 {
    a + 10
}

#[ctor::ctor]
fn init() {
    std::thread::spawn(|| {
        // TODO: find a better way to wait for the game to start
        std::thread::sleep(Duration::from_secs(1));

        register_function!(testing);
    });
}
