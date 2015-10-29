fn main() {
    let mut stack_grows_up = false;
    let platform = 
        if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
            "x86_64-linux"
        }
        else {
            panic!("unsupported platform")
        };

    println!("cargo:rustc-link-search=native=./lib");
    println!("cargo:rustc-link-lib=static=context-{0}", platform);
    if stack_grows_up {
        println!("cargo:rustc-cfg=stack_grows_up");
    }
}