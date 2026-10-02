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

    let allowlist = include_str!("deps/allowlist.txt")
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();

    let builder = bindgen::Builder::default()
        .clang_arg("-std=c++20")
        .clang_arg(format!("-I{}", red4ext_include_dir.display()))
        .header("deps/wrapper.hpp")
        .parse_callbacks(Box::new(Callbacks::default()))
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .derive_default(true)
        .enable_cxx_namespaces()
        .wrap_static_fns(true)
        .vtable_generation(true)
        .generate_comments(false)
        .layout_tests(false)
        // std types get generated incorrectly for some reason, so they need to be opaque
        .opaque_type("std::(vector|string|filesystem).*")
        .allowlist_item("versioning::.+");

    let builder = allowlist
        .iter()
        .fold(builder, bindgen::Builder::allowlist_item);

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    #[cfg(debug_assertions)]
    println!(
        "cargo:warning=Generated bindings: {}",
        out_path.join("bindings.rs").display()
    );
}

#[derive(Debug, Default)]
struct Callbacks(bindgen::CargoCallbacks);

impl bindgen::callbacks::ParseCallbacks for Callbacks {
    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        if [
            "CClass",
            "CBaseFunction",
            "CGlobalFunction",
            "CClassFunction",
            "CClassStaticFunction",
        ]
        .contains(&info.name)
        {
            vec!["Debug".to_string()]
        } else {
            vec![]
        }
    }

    fn header_file(&self, filename: &str) {
        self.0.header_file(filename);
    }

    fn include_file(&self, filename: &str) {
        self.0.include_file(filename);
    }

    fn read_env_var(&self, key: &str) {
        self.0.read_env_var(key);
    }
}
