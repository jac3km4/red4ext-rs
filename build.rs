use std::env;
use std::path::PathBuf;

fn main() {
    // `cfg!(target_os)` in a build script is the host, so the target comes from cargo's environment.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let red4ext_dir = if target_os == "macos" {
        // macOS: the pinned deps/RED4ext.SDK is upstream and has no macOS support (it includes Windows.h
        // everywhere), so the SDK comes from a RED4ext.SDK-macos checkout (branch mac-rs). Evidence: B1 report E0.
        println!("cargo:rerun-if-env-changed=RED4EXT_SDK_DIR");
        match env::var_os("RED4EXT_SDK_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => panic!(
                "RED4EXT_SDK_DIR is not set: on macOS red4ext-rs needs a RED4ext.SDK-macos checkout \
                 (branch mac-rs), e.g. RED4EXT_SDK_DIR=$HOME/repos/cp2077-mac/RED4ext.SDK-macos-rs"
            ),
        }
    } else {
        PathBuf::from("deps/RED4ext.SDK")
    };
    let red4ext_include_dir = red4ext_dir.join("include");

    let red4ext_target = cmake::Config::new(&red4ext_dir).profile("Release").build();

    println!(
        "cargo:rustc-link-search=native={}",
        red4ext_target.join("lib").display()
    );
    if target_os == "windows" {
        println!("cargo:rustc-link-lib=user32");
    }
    println!("cargo:rustc-link-lib=RED4ext.SDK");
    if target_os == "macos" {
        // The SDK static lib is C++ and needs the libc++ runtime (__cxa_*, __gxx_personality_v0). B1 report E5.
        println!("cargo:rustc-link-lib=c++");

        // The Mac SDK header routes nine hashes to plugin-provided tweakxl_* symbols (memory vault, RTTI id counters, CClass
        // virtual slots). B1 report E5 lists them as the only undefined symbols left in the link.
        println!("cargo:rerun-if-changed=deps/macos/MacVault.cpp");
        cc::Build::new()
            .cpp(true)
            .cpp_link_stdlib(None)
            .std("c++20")
            .include(&red4ext_include_dir)
            .define("RED4EXT_STATIC_LIB", None)
            .warnings(false)
            .file("deps/macos/MacVault.cpp")
            .compile("r4e_mac_vault");
    }

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
