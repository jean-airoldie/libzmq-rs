use libzmq::{prelude::*, *};

use std::thread;

fn main() -> Result<(), failure::Error> {
    let addr: InprocAddr = InprocAddr::new_unique();

    let server = ServerBuilder::new()
        .bind(&addr)
        .build()?;

    // Spawn the server thread.
    let handle = thread::spawn(move || -> Result<(), Error> {
        loop {
            let request = server.recv_msg()?;
            assert_eq!(request.to_str(), Ok("ping"));

            // Retrieve the routing_id to route the reply to the client.
            let id = request.routing_id().unwrap();
            let mut reply: Msg = "pong".into();
            reply.set_routing_id(id);
            // We cast the Error<Msg> to Error<()>. This drops the Msg.
            server.send(reply).map_err(Error::cast)?;
        }
    });

    let client = ClientBuilder::new()
        .connect(addr)
        .build()?;

    // Do some request-reply work.
    client.send("ping")?;
    let msg = client.recv_msg()?;
    assert_eq!(msg.to_str(), Ok("pong"));

    // This will cause the server to fail with `CtxTerminated`.
    Ctx::global().shutdown();

    // Join with the thread.
    let err = handle.join().unwrap().unwrap_err();
    assert_eq!(err.kind(), ErrorKind::CtxTerminated);

    Ok(())
}
