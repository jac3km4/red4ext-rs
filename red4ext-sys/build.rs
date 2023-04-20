use std::io::Write;
use std::path::{Path, PathBuf};

// method borrowed from [rust-snappy](https://github.com/BurntSushi/rust-snappy/blob/master/build.rs)
fn write_crc_tables(out_dir: &Path) -> std::io::Result<()> {
    let out_path = out_dir.join("crc32_table.rs");
    let mut out = std::io::BufWriter::new(std::fs::File::create(out_path)?);

    let table = get_table();

    writeln!(out, "pub const TABLE: [u32; 256] = [")?;
    for &x in table.iter() {
        writeln!(out, "    {},", x)?;
    }
    writeln!(out, "];")?;

    out.flush()?;

    Ok(())
}

// code borrowed from [const-crc32](https://git.shipyard.rs/jstrong/const-crc32/src/branch/master/LICENSE)
const fn table_fn(i: u32) -> u32 {
    let mut out = i;

    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };
    out = if out & 1 == 1 {
        0xedb88320 ^ (out >> 1)
    } else {
        out >> 1
    };

    out
}

// code borrowed from [const-crc32](https://git.shipyard.rs/jstrong/const-crc32/src/branch/master/LICENSE)
const fn get_table() -> [u32; 256] {
    let mut table: [u32; 256] = [0u32; 256];
    let mut i = 0;

    while i < 256 {
        table[i] = table_fn(i as u32);
        i += 1;
    }

    table
}

fn main() {
    let out_dir = match std::env::var_os("OUT_DIR") {
        None => {
            panic!("OUT_DIR environment variable not defined");
        }
        Some(out_dir) => PathBuf::from(out_dir),
    };
    let _unused = write_crc_tables(&out_dir);

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
