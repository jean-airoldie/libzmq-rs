use libzmq::{prelude::*, *};

use std::{convert::TryInto, thread, time::Duration};

fn main() -> Result<(), failure::Error> {
    // We use a system assigned port here.
    let addr: TcpAddr = "127.0.0.1:*".try_into()?;

    let duration = Duration::from_millis(300);
    let hb = Heartbeat::new(duration)
        .timeout(2 * duration)
        .ttl(2 * duration);

    let server = ServerBuilder::new()
        .bind(addr)
        .send_timeout(duration)
        .heartbeat(&hb)
        .build()?;

    // Retrieve the assigned port.
    let bound = server.last_endpoint()?.unwrap();

    // Spawn the server thread.
    let handle = thread::spawn(move || -> Result<(), Error> {
        use ErrorKind::*;
        loop {
            let request = server.recv_msg()?;
            assert_eq!(request.to_str(), Ok("ping"));

            // Retrieve the routing_id to route the reply to the client.
            let id = request.routing_id().unwrap();
            let mut reply: Msg = "pong".into();
            reply.set_routing_id(id);

            if let Err(err) = server.send(reply) {
                match err.kind() {
                    // Cannot route msg, drop it.
                    WouldBlock | HostUnreachable => (),
                    _ => return Err(err.cast()),
                }
            }
        }
    });

    let client = ClientBuilder::new()
        .connect(bound)
        .recv_timeout(duration)
        .send_timeout(duration)
        .heartbeat(hb)
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
