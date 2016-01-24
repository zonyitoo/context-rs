extern crate gcc;

use std::path::PathBuf;
use std::env;

const LIB_NAME: &'static str = "libctxswtch.a";

fn main() {
    let target = env::var("TARGET").unwrap();
    let arch_sub = target.split('-').next().unwrap();

    let arch =
        match arch_sub {
            "x86_64" => "x86_64",
            "x86" | "i686" | "i586" | "i486" | "i386" => "i686",
            "arm" => "arm",
            "mips" => "mips",
            "mipsel" => "mipsel",
            _ => {
                panic!("Unsupported architecture: {}", target);
            }
        };

    let src_path = &["src", "asm", arch, "_context.S"].iter().collect::<PathBuf>();
    gcc::compile_library(LIB_NAME, &[src_path.to_str().unwrap()]);

// seems like this line is no need actually
//    println!("cargo:rustc-flags=-l ctxswtch:static");
}
