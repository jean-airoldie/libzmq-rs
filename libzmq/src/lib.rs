#![doc(html_root_url = "https://docs.rs/libzmq/0.1.0")]

//! libzmq - A strict subset of ØMQ with a high level API.

pub use failure;

#[macro_use]
mod core;
pub mod addr;
pub mod auth;
pub mod ctx;
mod error;
pub mod group;
mod msg;
mod old;
pub mod poll;
pub mod socket;
mod utils;

#[doc(inline)]
pub use addr::{EpgmAddr, InprocAddr, PgmAddr, TcpAddr, UdpAddr};
#[doc(inline)]
pub use ctx::Ctx;
pub use error::{Error, ErrorKind};
#[doc(inline)]
pub use group::{Group, GroupOwned};
pub use msg::*;
pub use socket::{
    Client, ClientBuilder, Dish, DishBuilder, Radio, RadioBuilder, Server,
    ServerBuilder,
};
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

#[cfg(test)]
mod test {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("../README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
