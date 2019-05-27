use libzmq::{prelude::*, *};

use std::thread;

// This is a simple as it gets. No send_timeout or
// recv_timeout, no good error handling. For a `INPROC`
// server, this works.

fn main() -> Result<(), failure::Error> {
    let addr: InprocAddr = InprocAddr::new_unique();

    let server = ServerBuilder::new()
        .bind(&addr)
        .build()?;

    // Spawn the server thread.
    let handle = thread::spawn(move || -> Result<(), failure::Error> {
        loop {
            let request = server.recv_msg()?;
            // We define a empty message as a termination signal.
            if request.is_empty() {
                break Ok(());
            } else {
                assert_eq!(request.to_str(), Ok("ping"));
            }

            // Retrieve the routing_id to route the reply to the client.
            let id = request.routing_id().unwrap();
            let mut reply: Msg = "pong".into();
            reply.set_routing_id(id);
            server.send(reply)?;
        }
    });

    let client = ClientBuilder::new()
        .connect(addr)
        .build()?;

    // Do some request-reply work.
    client.send("ping")?;
    let msg = client.recv_msg()?;
    assert_eq!(msg.to_str(), Ok("pong"));

    // Send the termination signal.
    client.send("")?;

    // Join with the thread.
    handle.join().unwrap()
}
