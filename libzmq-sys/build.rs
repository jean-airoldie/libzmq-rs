use std::env;
#[cfg(feature = "renew-bindings")]
use std::path::{Path, PathBuf};

#[cfg(feature = "renew-bindings")]
fn gen_bindings(include_dir: &Path) {
    let args = vec!["-DZMQ_BUILD_DRAFT_API=1"];

    let bindings = bindgen::Builder::default()
        .header(include_dir.join("zmq.h").to_string_lossy())
        .derive_default(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .derive_debug(true)
        .derive_hash(true)
        .whitelist_function("^zmq_.*")
        .whitelist_type("^zmq_.*")
        .whitelist_var("^ZMQ_.*")
        .clang_args(args)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from("./src").join("bindings.rs");
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PROFILE");

    let wants_debug = env::var_os("PROFILE").unwrap() == "debug";

    let artifacts = zeromq_src::Build::new()
        .link_static(true)
        .enable_draft(true)
        .build_debug(wants_debug)
        .build();

    artifacts.print_cargo_metadata();

    #[cfg(feature = "renew-bindings")]
    gen_bindings(artifacts.include_dir());
}
