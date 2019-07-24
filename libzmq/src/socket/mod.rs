//! The ØMQ socket types.

mod client;
mod dish;
mod gather;
mod radio;
mod scatter;
mod server;

pub use client::*;
pub use dish::*;
pub use gather::*;
pub use radio::*;
pub use scatter::*;
pub use server::*;

use crate::{
    core::{AsRawSocket, RawSocket},
    Error,
};

use serde::{Deserialize, Serialize};

/// An enum containing all the socket types.
///
/// # Note
/// This error type is non-exhaustive and could have additional variants
/// added in future. Therefore, when matching against variants of
/// non-exhaustive enums, an extra wildcard arm must be added to account
/// for any future variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketType {
    Client(Client),
    Server(Server),
    Radio(Radio),
    Dish(Dish),
    Gather(Gather),
    Scatter(Scatter),
}

impl AsRawSocket for SocketType {
    fn raw_socket(&self) -> &RawSocket {
        match self {
            SocketType::Client(client) => client.raw_socket(),
            SocketType::Server(server) => server.raw_socket(),
            SocketType::Radio(radio) => radio.raw_socket(),
            SocketType::Dish(dish) => dish.raw_socket(),
            SocketType::Gather(dish) => dish.raw_socket(),
            SocketType::Scatter(dish) => dish.raw_socket(),
        }
    }
}

/// An enum containing all the socket config types.
///
/// # Note
/// This error type is non-exhaustive and could have additional variants
/// added in future. Therefore, when matching against variants of
/// non-exhaustive enums, an extra wildcard arm must be added to account
/// for any future variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigType {
    Client(ClientConfig),
    Server(ServerConfig),
    Radio(RadioConfig),
    Dish(DishConfig),
    Gather(GatherConfig),
    Scatter(ScatterConfig),
}

impl ConfigType {
    pub fn build(&self) -> Result<SocketType, Error<usize>> {
        match self {
            ConfigType::Client(config) => {
                let client = config.build()?;
                Ok(SocketType::Client(client))
            }
            ConfigType::Server(config) => {
                let server = config.build()?;
                Ok(SocketType::Server(server))
            }
            ConfigType::Radio(config) => {
                let radio = config.build()?;
                Ok(SocketType::Radio(radio))
            }
            ConfigType::Dish(config) => {
                let dish = config.build()?;
                Ok(SocketType::Dish(dish))
            }
            ConfigType::Gather(config) => {
                let dish = config.build()?;
                Ok(SocketType::Gather(dish))
            }
            ConfigType::Scatter(config) => {
                let dish = config.build()?;
                Ok(SocketType::Scatter(dish))
            }
        }
    }
}
