extern crate gcc;

use std::path::PathBuf;
use std::env;

const LIB_NAME: &'static str = "libctxswtch.a";

fn main() {
    // Use new layout on Linux x86_64
    if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
        println!("cargo:rustc-link-search=native=./lib");
        return;
    }
    // Otherwise, use old binaries
    let arch =
        if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "x86") {
            "i686"
        } else if cfg!(target_arch = "arm") {
            "arm"
        } else if cfg!(target_arch = "mips") {
            "mips"
        } else if cfg!(target_arch = "mipsel") {
            "mipsel"
        } else {
            panic!("Unsupported architecture: {}", env::var("TARGET").unwrap());
        };
    let src_path = &["src", "asm", arch, "_context.S"].iter().collect::<PathBuf>();
    gcc::compile_library(LIB_NAME, &[src_path.to_str().unwrap()]);

// seems like this line is no need actually
//    println!("cargo:rustc-flags=-l ctxswtch:static");
}
