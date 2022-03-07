use std::path::PathBuf;

fn main() {
    let includes: &[PathBuf] = &[
        PathBuf::from("deps").join("RED4ext.SDK").join("include"),
        PathBuf::from("deps").join("glue"),
    ];

    cxx_build::bridge("src/lib.rs")
        .includes(includes)
        .compiler("clang")
        .flag("-std=c++20")
        .flag("-D_DLL")
        .flag("-Wno-everything")
        .compile("red4ext-rs");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=deps/glue/glue.hpp");
}
