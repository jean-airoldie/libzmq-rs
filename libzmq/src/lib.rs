#![doc(html_root_url = "https://docs.rs/libzmq/0.2")]

//! *libzmq* - A strict subset of ØMQ with a high level API.

pub use failure;

#[macro_use]
mod core;
pub mod auth;
mod ctx;
mod endpoint;
mod error;
mod group;
mod msg;
mod old;
pub mod poll;
mod socket;
mod utils;

pub use crate::core::{Heartbeat, Period};
pub use ctx::{Ctx, CtxBuilder, CtxHandle};
pub use endpoint::{
    EpgmAddr, InprocAddr, PgmAddr, TcpAddr, UdpAddr, INPROC_MAX_SIZE,
};
pub use error::{Error, ErrorKind};
pub use group::*;
pub use msg::*;
pub use socket::{
    Client, ClientBuilder, Dish, DishBuilder, Gather, GatherBuilder, Radio,
    RadioBuilder, Scatter, ScatterBuilder, Server, ServerBuilder, SocketType,
};
pub use utils::*;
/// Configurations for *libzmq* types.
pub mod config {
    pub use crate::auth::client::AuthConfig;
    pub use crate::ctx::CtxConfig;
    pub use crate::socket::{
        ClientConfig, ConfigType, DishConfig, GatherConfig, RadioConfig,
        ScatterConfig, ServerConfig,
    };
}

/// Address related types.
pub mod addr {
    pub use crate::endpoint::{
        AddrParseError, Endpoint, Hostname, Interface, IntoIpAddrs, Port,
        SocketAddr, SrcAddr,
    };
}

/// A "prelude" for users of the *libzmq* crate.
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
    pub use crate::core::{
        BuildHeartbeating, BuildRecv, BuildSend, BuildSocket,
        ConfigureHeartbeating, ConfigureRecv, ConfigureSend, ConfigureSocket,
        Heartbeating, RecvMsg, SendMsg, Socket,
    };
    // These should be in the standard prelude anyway.
    pub use std::convert::{TryFrom, TryInto};
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
