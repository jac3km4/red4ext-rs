use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let red4ext_dir = Path::new("deps/RED4ext.SDK");
    let red4ext_include_dir = red4ext_dir.join("include");

    let red4ext_target = cmake::Config::new(red4ext_dir).profile("Release").build();

    println!(
        "cargo:rustc-link-search=native={}",
        red4ext_target.join("lib").display()
    );
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=RED4ext.SDK");

    let bindings = bindgen::Builder::default()
        .clang_arg("-std=c++20")
        .clang_arg(format!("-I{}", red4ext_include_dir.display()))
        .header("deps/wrapper.hpp")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .derive_default(true)
        .enable_cxx_namespaces()
        .wrap_static_fns(true)
        .vtable_generation(true)
        // std types get generated incorrectly for some reason, so they need to be opaque
        .opaque_type("std::(vector|string)")
        .allowlist_item("RED4ext::[^:]+")
        .allowlist_item("RED4ext::(Detail|ent)::.+")
        .allowlist_item("RED4ext::Memory::(Vault|IAllocator)")
        .allowlist_item("versioning::.+")
        // callback handlers generate incorrect Rust code
        .blocklist_item("RED4ext::(Detail::)?CallbackHandler.*")
        .generate_comments(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    #[cfg(debug_assertions)]
    println!(
        "cargo:warning=Generated bindings: {}",
        out_path.join("bindings.rs").display()
    );

    const SIZE: usize = 512;
    let mut sizes = String::with_capacity(11264); // increase according to size (SIZE)
    sizes.push_str("pub const fn const_digit_str(x: usize) -> &'static str {");
    sizes.push_str("\n    match x {");
    for i in 1..=SIZE {
        sizes.push_str("\n        ");
        sizes.push_str(i.to_string().as_str());
        sizes.push_str(" => ");
        sizes.push('"');
        sizes.push_str(i.to_string().as_str());
        sizes.push('"');
        sizes.push(',');
    }
    sizes.push_str("\n      _ => unimplemented!()");
    sizes.push_str("\n    }");
    sizes.push('\n');
    sizes.push('}');
    std::fs::write(out_path.join("sizes.rs"), sizes).expect("Couldn't write const_digit_str!");
}
