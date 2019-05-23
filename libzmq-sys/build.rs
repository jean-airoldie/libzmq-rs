use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let wants_debug = cfg!(debug_assertions);

    let artifacts = zeromq_src::Build::new()
        .link_static(true)
        .enable_draft(true)
        .build_debug(wants_debug)
        .build();
    let args = vec!["-DZMQ_BUILD_DRAFT_API=1"];

    let bindings = bindgen::Builder::default()
        .header(artifacts.include_dir().join("zmq.h").to_string_lossy())
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
    artifacts.print_cargo_metadata();
}
