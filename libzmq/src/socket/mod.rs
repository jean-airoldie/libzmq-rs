//! The ØMQ socket types.

mod client;
mod dish;
mod radio;
mod server;

pub use client::*;
pub use dish::*;
pub use radio::*;
pub use server::*;

use crate::{
    core::{GetRawSocket, RawSocket},
};

use serde::{Deserialize, Serialize};

/// An enum containing all the socket types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketType {
    Client(Client),
    Server(Server),
    Radio(Radio),
    Dish(Dish),
}

impl GetRawSocket for SocketType {
    fn raw_socket(&self) -> &RawSocket {
        match self {
            SocketType::Client(client) => client.raw_socket(),
            SocketType::Server(server) => server.raw_socket(),
            SocketType::Radio(radio) => radio.raw_socket(),
            SocketType::Dish(dish) => dish.raw_socket(),
        }
    }
}

/// An enum containing all the socket config types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigType {
    Client(ClientConfig),
    Server(ServerConfig),
    Radio(RadioConfig),
    Dish(DishConfig),
}

impl ConfigType {
    pub fn build(&self) -> Result<SocketType, failure::Error> {
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
        }
    }
}
