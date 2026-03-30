// build.rs — Copy SDL2.dll from MSYS2 to the output directory automatically
use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // OUT_DIR is something like target/<triple>/debug/build/<pkg>/out
    // The executable lives three levels up
    let exe_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("Could not find executable directory")
        .to_path_buf();

    let sdl2_dll = PathBuf::from("C:/msys64/ucrt64/bin/SDL2.dll");
    if sdl2_dll.exists() {
        let dest = exe_dir.join("SDL2.dll");
        let _ = std::fs::copy(&sdl2_dll, &dest);
    }

    println!("cargo:rustc-link-search=native=C:/msys64/ucrt64/lib");
    println!("cargo:rerun-if-changed=build.rs");
}
