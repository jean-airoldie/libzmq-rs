[![Apache 2.0 licensed](https://img.shields.io/badge/license-Apache2.0-blue.svg)](./LICENSE-APACHE)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)

> libzmq-rs

A strict subset of ØMQ with a high level API.

# Cargo
```toml
[dependencies]
libzmq = "0.1"
```

# Installation
This crate builds and generates bindings from source. This means that you
do not need to install `libzmq`. However building from source requires:
* [CMake 2.8.12+ (or 3.0.2+ on Darwin)](https://github.com/zeromq/libzmq/blob/de4d69f59788fed86bcb0f610723c5acd486a7da/CMakeLists.txt#L7)
* [Clang 6.0+](https://github.com/rust-lang/rust-bindgen/blob/master/Cargo.toml#L51)

# Linking
By default `libzmq` is built and linked dynamically. To change this behavior
[read this](./libzmq-sys/README.md).

# General Goals
* Conform to these [`API guidelines`].
* Prevent footguns (which are plentifull in `libzmq`)
* Minimize the learning curve
* Don't sacrifice any performance
* Extensively document

To do so we will only use a subset of `libzmq`. If you'd rather have a complete
port, check out [`rust-zmq`].

# Stability Guarantees
There are no stability guarantees until the `1.0` version of the API is released.
Thus expected a lot of breaking changes until then. Furthermore, since a large part of
the library relies on ØMQ's DRAFT API, they will have to be stabilized before the 1.0
version is released.

# Frequently Asked Questions
See the [`FAQ`](./FAQ.md).

# Aknowledgements
* A parts of the code was based on [`rust-zmq`].

[`rust-zmq`]: https://github.com/erickt/rust-zmq
[`API guidelines`]: https://rust-lang-nursery.github.io/api-guidelines/checklist.html
[`libzmq`]: https://github.com/zeromq/libzmq
