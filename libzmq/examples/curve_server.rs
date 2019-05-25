use libzmq::{config::*, prelude::*};

use serde::{Deserialize, Serialize};

use std::{fs::File, io::Read};

// Take a look at the config file.
const CONFIG_PATH: &str = "examples/curve_server.yml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    auth: AuthConfig,
    client: ClientConfig,
    server: ServerConfig,
}

fn read_file(name: &str) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(name)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn main() -> Result<(), failure::Error> {
    let config: Config =
        serde_yaml::from_slice(&read_file(CONFIG_PATH).unwrap()).unwrap();

    // Create a `AuthClient` and transmits the configuration to the
    // background `AuthServer` thread.
    let _ = config.auth.build()?;

    let client = config.client.build()?;
    let server = config.server.build()?;

    // In this example the server binds to a system defined port so as
    // prevent potential conflicts with the host machine. But in a real
    // application the port would be known (and added to the config file).
    let bound = server.last_endpoint()?;
    client.connect(bound)?;

    // Do some request reply work.
    client.send("hello")?;

    let msg = server.recv_msg()?;
    server.send(msg)?;

    let _ = client.recv_msg()?;

    Ok(())
}
