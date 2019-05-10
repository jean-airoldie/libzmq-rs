#![doc(html_root_url = "https://docs.rs/libzmq/0.1")]

//! libzmq - A strict subset of ØMQ with a high level API.

pub use failure;

#[macro_use]
mod core;
pub mod addr;
pub mod ctx;
mod error;
pub mod group;
mod msg;
pub mod poll;
pub mod socket;
mod utils;

pub use addr::Endpoint;
pub use ctx::Ctx;
pub use error::{Error, ErrorKind};
pub use group::{Group, GroupOwned};
pub use msg::*;
pub use socket::{Client, Dish, Radio, Server};
pub use utils::*;

/// A "prelude" for users of the `ØMQ` crate.
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
