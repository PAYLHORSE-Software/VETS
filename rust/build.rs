extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=static=kakasi");
    println!("cargo:rustc-link-search=native=./libs");

    let kakasi_include_path = "./c_bindings";

    let bindings = bindgen::Builder::default()
        .header(format!("{}/libkakasi.h", kakasi_include_path))
        .clang_arg(format!("-I{}", kakasi_include_path))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("kakasi_bindings.rs"))
        .expect("Couldn't write bindings!");
}
