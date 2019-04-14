#![doc(html_root_url = "https://docs.rs/libzmq/0.1")]

//! libzmq - A strict subset of ØMQ with a high level API.

#[macro_use]
mod core;
pub mod ctx;
pub mod endpoint;
mod error;
mod msg;
pub mod poll;
pub mod types;
mod utils;

pub use ctx::Ctx;
pub use endpoint::Endpoint;
pub use error::{Error, ErrorKind};
pub use msg::Msg;
pub use poll::{Poller, INCOMING, NO_EVENTS, OUTGOING};
pub use types::{Client, Dish, Radio, Server};
pub use utils::*;

/// A "prelude" for users of the `libzmq` crate.
///
/// This prelude is similar to the standard library's prelude in that you'll
/// almost always want to import its entire contents, but unlike the standard
/// library's prelude you'll have to do so manually:
///
/// ```
/// use libzmq::prelude::*;
/// ```
///
/// The prelude may grow over time as additional items see ubiquitous use.
pub mod prelude {
    pub use crate::core::*;
}
