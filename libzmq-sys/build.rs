use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=LIBZMQ_SYS_STATIC");
    println!("cargo:rerun-if-changed=build.rs");

    let wants_static = cfg!(feature = "static")
        || env::var("LIBZMQ_SYS_STATIC").unwrap_or_default() == "1";

    let wants_draft = cfg!(feature = "draft");

    let dest = {
        let mut config = cmake::Config::new("vendor");

        if cfg!(feature = "draft") {
            config.define("ENABLE_DRAFTS", "ON");
        } else {
            config.define("ENABLE_DRAFTS", "OFF");
        }

        if wants_static {
            let dest = config
                .define("BUILD_SHARED", "OFF")
                .define("BUILD_STATIC", "ON")
                .build();

            // The exact location seem to vary by system.
            println!(
                "cargo:rustc-link-search=native={}",
                dest.join("lib64").to_string_lossy()
            );
            println!(
                "cargo:rustc-link-search=native={}",
                dest.join("lib").to_string_lossy()
            );
            println!("cargo:rustc-link-lib=static=zmq");

            dest
        } else {
            let dest = config
                .define("BUILD_SHARED", "ON")
                .define("BUILD_STATIC", "OFF")
                .build();

            // The exact location seem to vary by system.
            println!(
                "cargo:rustc-link-search=native={}",
                dest.join("lib64").to_string_lossy()
            );
            println!(
                "cargo:rustc-link-search=native={}",
                dest.join("lib").to_string_lossy()
            );
            println!("cargo:rustc-link-lib=dylib=zmq");

            dest
        }
    };
    println!("cargo:rustc-link-lib=stdc++");

    println!("cargo:include={}", dest.join("include").to_string_lossy());
    println!("cargo:root={}", dest.to_string_lossy());
    println!(
        "cargo:pkg-config-path={}",
        dest.join("lib64").join("pkgconfig").to_string_lossy()
    );

    let mut args = vec![];

    if wants_draft {
        args.push("-DZMQ_BUILD_DRAFT_API=1");
    }

    let bindings = bindgen::Builder::default()
        .header(dest.join("include").join("zmq.h").to_string_lossy())
        .derive_default(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .derive_debug(true)
        .derive_hash(true)
        .clang_args(args)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
