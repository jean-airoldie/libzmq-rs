use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=LIBZMQ_SYS_STATIC");
    println!("cargo:rerun-if-env-changed=LIBZMQ_SYS_DEBUG");
    println!("cargo:rerun-if-changed=build.rs");

    let wants_static = cfg!(feature = "static")
        || env::var("LIBZMQ_SYS_STATIC").unwrap_or_default() == "1";

    let wants_debug = cfg!(feature = "debug")
        || env::var("LIBZMQ_SYS_DEBUG").unwrap_or_default() == "1";

    let wants_draft = cfg!(feature = "draft");

    let artifacts = zeromq_src::Build::new()
        .link_static(wants_static)
        .build_debug(wants_debug)
        .build();
    let mut args = vec![];

    if wants_draft {
        args.push("-DZMQ_BUILD_DRAFT_API=1");
    }

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
