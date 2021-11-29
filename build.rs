use std::path::PathBuf;

fn main() {
    let includes: &[PathBuf] = &[
        PathBuf::from("deps").join("RED4ext.SDK").join("include"),
        PathBuf::from("deps").join("glue"),
    ];

    let mut build = autocxx_build::Builder::new("src/lib.rs", includes)
        .extra_clang_args(&["-std=c++20"])
        .expect_build();
    build.flag("-std:c++20").compile("red4ext-rs");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
