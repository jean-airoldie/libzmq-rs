use libzmq::config::*;
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    auth: AuthConfig,
    client: ClientConfig,
    server: ServerConfig,
}

fn main() -> Result<(), failure::Error> {
    Ok(())
}
