[![](https://img.shields.io/crates/v/libzmq.svg)][crates-io]
[![](https://docs.rs/libzmq/badge.svg)][api-docs]
[![Apache 2.0 licensed](https://img.shields.io/badge/license-Apache2.0-blue.svg)](./LICENSE-APACHE)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)

> libzmq-rs

A strict subset of ØMQ with an ergonomic API.

# Installation
This crate builds and generates bindings from source. This means that you
do not need to install [`libzmq`]. However building from source requires:
* [CMake 2.8.12+ (or 3.0.2+ on Darwin)](https://github.com/zeromq/libzmq/blob/de4d69f59788fed86bcb0f610723c5acd486a7da/CMakeLists.txt#L7)
* [Clang 6.0+](https://github.com/rust-lang/rust-bindgen/blob/master/Cargo.toml#L51)

# License
This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

[crates-io]: https://crates.io/crates/libzmq
[api-docs]: https://docs.rs/libzmq
