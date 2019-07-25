use libzmq::{config::*, prelude::*, *};

use serde::{Deserialize, Serialize};

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    thread,
};

const CONFIG_FILE: &str = "secure_req_rep.yml";

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

fn main() -> Result<(), failure::Error> {
    let path = PathBuf::from("examples").join(CONFIG_FILE);

    let config: Config =
        serde_yaml::from_slice(&read_file(&path).unwrap()).unwrap();

    // Configure the `AuthServer`. We won't need the returned `AuthClient`.
    let _ = config.auth.build()?;

    // Configure our two sockets.
    let server = config.server.build()?;
    let client = config.client.build()?;

    // Once again we used a system assigned port for our server.
    let bound = server.last_endpoint()?;
    client.connect(bound)?;

    // Spawn the server thread. In a real application, this
    // would be on another node.
    let handle = thread::spawn(move || -> Result<(), Error> {
        use ErrorKind::*;
        loop {
            let request = server.recv_msg()?;
            assert_eq!(request.to_str(), Ok("ping"));

            // Retrieve the routing_id to route the reply to the client.
            let id = request.routing_id().unwrap();
            if let Err(err) = server.route("pong", id) {
                match err.kind() {
                    // Cannot route msg, drop it.
                    WouldBlock | HostUnreachable => (),
                    _ => return Err(err.cast()),
                }
            }
        }
    });

    // Do some request-reply work.
    client.send("ping")?;
    let msg = client.recv_msg()?;
    assert_eq!(msg.to_str(), Ok("pong"));

    // This will cause the server to fail with `InvalidCtx`.
    Ctx::global().shutdown();

    // Join with the thread.
    let err = handle.join().unwrap().unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidCtx);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn main_runs() {
        main().unwrap();
    }
}
