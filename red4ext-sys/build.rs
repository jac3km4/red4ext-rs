use std::path::PathBuf;

fn main() {
    let includes: &[PathBuf] = &[
        PathBuf::from("cpp").join("RED4ext.SDK").join("include"),
        PathBuf::from("cpp").join("glue"),
    ];

    cxx_build::bridge("src/lib.rs")
        .includes(includes)
        .flag("-std:c++20")
        .compile("red4ext-rs");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cpp/glue/glue.hpp");
}
