use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    let red4ext_dir = Path::new("deps/RED4ext.SDK");
    let red4ext_include_dir = red4ext_dir.join("include");

    println!("cargo:rerun-if-changed=deps/wrapper.hpp");
    println!("cargo:rerun-if-changed=deps/allowlist.txt");
    println!("cargo:rerun-if-changed=deps/bridge");

    // Bindgen tracks headers; Cargo must also watch CMake's other inputs.
    for input in ["CMakeLists.txt", "cmake", "src", "vendor"] {
        println!(
            "cargo:rerun-if-changed={}",
            red4ext_dir.join(input).display()
        );
    }
    println!("cargo:rerun-if-env-changed=RED4EXT_USE_PCH");
    println!("cargo:rerun-if-env-changed=RED4EXT_BUILD_TIMINGS");
    let timings = env::var_os("RED4EXT_BUILD_TIMINGS").is_some_and(|value| value == "1");
    let use_pch = match env::var("RED4EXT_USE_PCH").as_deref() {
        Ok("0") => "OFF",
        Ok("1") | Err(env::VarError::NotPresent) => "ON",
        _ => panic!("RED4EXT_USE_PCH must be 0 or 1"),
    };
    let native_start = Instant::now();
    let red4ext_target = cmake::Config::new(red4ext_dir)
        .profile("Release")
        .define("RED4EXT_USE_PCH", use_pch)
        .build();
    if timings {
        println!(
            "cargo:warning=SDK configure/build/install (PCH {use_pch}): {:?}",
            native_start.elapsed()
        );
    }

    println!(
        "cargo:rustc-link-search=native={}",
        red4ext_target.join("lib").display()
    );
    let bridge_target = cmake::Config::new("deps/bridge")
        .out_dir(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bridge"))
        .profile("Release")
        .define(
            "RED4EXT_SDK_DIR",
            env::current_dir()
                .expect("working directory must exist")
                .join(red4ext_dir),
        )
        .build();
    println!(
        "cargo:rustc-link-search=native={}",
        bridge_target.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=red4ext_rs_bridge");
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=static=RED4ext.SDK");

    let allowlist = include_str!("deps/allowlist.txt")
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();

    let builder = bindgen::Builder::default()
        .time_phases(timings)
        .clang_arg("-std=c++20")
        .clang_arg("-DRED4EXT_STATIC_LIB")
        .clang_arg(format!("-I{}", red4ext_include_dir.display()))
        .clang_arg(format!(
            "-I{}",
            red4ext_dir.join("vendor/D3D12MemAlloc").display()
        ))
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

    let bindings_start = Instant::now();
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    if timings {
        println!(
            "cargo:warning=Bindgen generation/write: {:?}",
            bindings_start.elapsed()
        );
    }

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
