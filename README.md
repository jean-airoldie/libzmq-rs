[![](https://img.shields.io/crates/v/libzmq.svg)][crates-io]
[![](https://docs.rs/libzmq/badge.svg)][api-docs]
[![Apache 2.0 licensed](https://img.shields.io/badge/license-Apache2.0-blue.svg)](./LICENSE-APACHE)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![](https://img.shields.io/website/https/jean-airoldie.github.io/libzmq-rs.svg)][website]

> libzmq-rs

A strict subset of ØMQ with an ergonomic API.

# Versioning & Stability Guarantees
**Expect breaking changes with each release until `0.2`**. After that breaking change only on minor
version until `1.0`. Furthermore, since a large part of the library relies on ØMQ's DRAFT API,
they will have to be stabilized before the 1.0 version is released.

```toml
[dependencies]
libzmq = "0.1.12"
```

# Dead Simple Sample
```rust
use libzmq::{prelude::*, *};
use std::convert::TryInto;

// Use a system assigned port.
let addr: TcpAddr = "127.0.0.1:*".try_into()?;

let server = ServerBuilder::new()
    .bind(addr)
    .build()?;

// Retrieve the addr that was assigned.
let bound = server.last_endpoint()?;

let client = ClientBuilder::new()
    .connect(bound)
    .build()?;

// Send a string request.
client.send("tell me something")?;

// Receive the client request.
let msg = server.recv_msg()?;
let id = msg.routing_id().unwrap();

// Reply to the client.
let mut reply: Msg = "it takes 224 bits to store a i32 in java".into();
reply.set_routing_id(id);
server.send(reply)?;

// We can reply twice if we want.
let mut reply: Msg = "also don't talk to me".into();
reply.set_routing_id(id);
server.send(reply)?;

// Retreive the first reply.
let mut msg = client.recv_msg()?;
// And the second.
client.recv(&mut msg)?;
```

# Installation
This crate builds and generates bindings from source. This means that you
do not need to install [`libzmq`]. However building from source requires:
* [CMake 2.8.12+ (or 3.0.2+ on Darwin)](https://github.com/zeromq/libzmq/blob/de4d69f59788fed86bcb0f610723c5acd486a7da/CMakeLists.txt#L7)
* [Clang 6.0+](https://github.com/rust-lang/rust-bindgen/blob/master/Cargo.toml#L51)

# General Goals
* Conform to these [`API guidelines`].
* Provide an ergonomic API
* Prevent footguns (which are plentifull in [`libzmq`])
* Minimize the learning curve
* Don't sacrifice any performance
* Extensively document

To do so we will only use a subset of [`libzmq`]. If you'd rather have a complete
port, check out [`rust-zmq`].

# Frequently Asked Questions
See the [`FAQ`](./FAQ.md).

# Aknowledgements
* Based on [`rust-zmq`] and [`czmq`].

# License
This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `libzmq` by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[`rust-zmq`]: https://github.com/erickt/rust-zmq
[`czmq`]: https://github.com/zeromq/czmq
[`API guidelines`]: https://rust-lang-nursery.github.io/api-guidelines/checklist.html
[`libzmq`]: https://github.com/zeromq/libzmq
[crates-io]: https://crates.io/crates/libzmq
[api-docs]: https://docs.rs/libzmq
[website]: https://jean-airoldie.github.io/libzmq-rs/
