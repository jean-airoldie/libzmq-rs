# Basic Server

```rust
use libzmq::{prelude::*, *};
use std::{thread, convert::TryInto};

fn main() -> Result<(), failure::Error> {
    let addr: TcpAddr = "localhost:3000".try_into()?;

    let server = ServerBuilder::new()
        .bind(&addr)
        .build()?;

    let handle = thread::spawn(move || -> Result<(), failure::Error> {
        loop {
            let request = server.recv_msg()?;
            // Termination signal.
            if request.is_empty() {
                break;
            }

            assert_eq!(request.to_str(), Ok("ping"));
            let id = request.routing_id().unwrap();

            let mut reply: Msg = "pong".into();
            reply.set_routing_id(id);
            server.send(reply)?;
        }
    });

    let client = ClientBuilder::new()
        .connect(addr)
        .build()?;

    client.send("ping")?;
    let msg = client.recv_msg()?;
    assert_eq!(msg.to_str(), Ok("pong"));

    // Send termination signal.
    client.send("")?;

    handle.join().unwrap()
}
```
