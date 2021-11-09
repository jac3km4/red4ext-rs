use std::time::Duration;

use crate::prelude::*;

#[redscript]
fn to_lowercase(a: String) -> String {
    a.to_lowercase()
}

#[ctor::ctor]
fn init() {
    std::thread::spawn(|| {
        // TODO: find a better way to wait for the game to start
        std::thread::sleep(Duration::from_secs(1));

        register_function!("ToLowercase", to_lowercase);
    });
}
