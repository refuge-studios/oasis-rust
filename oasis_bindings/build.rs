/*
 * Example Code for the Oasis Graphics Framework
 * Copyright (c) 2025 REFUGE STUDIOS PTY LTD.
 * Created by Aidan Sanders <aidan.sanders@refugestudios.com.au>
 *
 * This example code is licensed under the MIT License.
 * You are free to use, modify, and distribute this code for any purpose,
 * including commercial applications, as long as this notice is retained.
 *
 * THE OASIS API ITSELF IS PROPRIETARY AND NOT COVERED UNDER THIS LICENSE.
 * These examples are intended to demonstrate usage of the Oasis API,
 * and require a licensed copy of Oasis to function.
 *
 * For licensing Oasis itself, please contact: aidan.sanders@refugestudios.com.au
 */
 
extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let lib_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..").join("lib");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=oasis");
    println!("cargo:rustc-link-arg=-Wl,-rpath={}", lib_dir.display());

    let bindings = bindgen::Builder::default()
        .header("include/oasis_c/oasis.h")
        .generate()
        .expect("Unable to generate bindings from oasis.h");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings to file");
}
