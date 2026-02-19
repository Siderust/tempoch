use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let include_dir = PathBuf::from(&crate_dir).join("include");

    let config =
        cbindgen::Config::from_file("cbindgen.toml").expect("Unable to read cbindgen.toml");

    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate C bindings");

    // Write to OUT_DIR (standard cargo location)
    bindings.write_to_file(out_dir.join("tempoch_ffi.h"));

    // Also write to include/ for easy consumption by C/C++ projects
    std::fs::create_dir_all(&include_dir).expect("Unable to create include directory");
    bindings.write_to_file(include_dir.join("tempoch_ffi.h"));
}
