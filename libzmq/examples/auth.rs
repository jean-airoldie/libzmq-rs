use libzmq::{config::*, prelude::*, Ctx};

use serde::{Deserialize, Serialize};

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

const CONFIG_FILES: &[&str] = &["curve.yml", "curve_no_auth.yml"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    auth: AuthConfig,
    client: ClientConfig,
    server: ServerConfig,
}

fn read_file(name: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(name)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn run(path: &Path) -> Result<(), failure::Error> {
    // We create a context since we want an isolated `AuthServer` for
    // our configuration.
    let ctx = Ctx::new();

    let config: Config =
        serde_yaml::from_slice(&read_file(path).unwrap()).unwrap();
    dbg!(&config);

    // Create a `AuthClient` and transmits the configuration to the
    // background `AuthServer` thread.
    let _ = config.auth.with_ctx(&ctx)?;

    let client = config.client.with_ctx(&ctx)?;
    let server = config.server.with_ctx(&ctx)?;

    // In case the server binds to a system defined port so as
    // prevent potential conflicts with the host machine.
    let bound = server.last_endpoint()?;
    client.connect(bound)?;

    // Do some request reply work.
    client.send("hello")?;

    let msg = server.recv_msg()?;
    server.send(msg)?;

    let _ = client.recv_msg()?;

    Ok(())
}

fn main() -> Result<(), failure::Error> {
    for filename in CONFIG_FILES {
        let path = PathBuf::from("examples").join(filename);
        run(&path)?;
    }

    Ok(())
}

// Make sure that the examples properly run since the config files are dynamic.
#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn main_runs() {
        main().unwrap();
    }
}
