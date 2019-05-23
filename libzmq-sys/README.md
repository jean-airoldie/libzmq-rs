[![Apache 2.0 licensed](https://img.shields.io/badge/license-Apache2.0-blue.svg)](./LICENSE-APACHE)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)

> libzmq-sys - Rust raw cffi bindings to libzmq

Based on this [`guide`](https://kornel.ski/rust-sys-crate) as well as [`zmq-sys`].

# Dependencies
* [CMake 2.8.12+ (or 3.0.2+ on Darwin)](https://github.com/zeromq/libzmq/blob/de4d69f59788fed86bcb0f610723c5acd486a7da/CMakeLists.txt#L7)
* [Clang 6.0+](https://github.com/rust-lang/rust-bindgen/blob/master/Cargo.toml#L51)

# Build and Linking.
The lib is built and linked statically.

# Build Type
The lib is built depending on the profile (either release or debug).

# OUTPUT ENV Variables
These are the output ENV variables of the cargo build script:
* `DEP_ZMQ_INCLUDE` is the directory which contains the `zmq.h` header.
* `DEP_ZMQ_ROOT` is the root of the `OUT_DIR`.
* `DEP_ZMQ_PKG_CONFIG_PATH` is the path to the directory
    containing the `libzmq.pc` package config fileo

[`guide`]: https://kornel.ski/rust-sys-crate
[`zmq-sys`]: https://github.com/erickt/rust-zmq/tree/master/zmq-sys
