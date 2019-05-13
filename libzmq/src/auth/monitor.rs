use crate::core::{GetRawSocket};
use libzmq_sys as sys;
use std::ffi::CString;

fn register_monitor_socket<S, E>(socket: &OldSocket, endpoint: E, events: MonitorEvents)
    where S: GetRawSocket, E: Into<Endpoint>
{
    let endpoint = endpoint.into();
    let c_string = CString::new(endpoint.to_zmq()).unwrap();

    let rc = unsafe { sys::zmq_socket_monitor(
        socket.raw_socket().as_mut_ptr(),
        c_string.as_ptr(),
        events.bytes(),
        )
    };

    assert_ne!(rc, -1);
}

/// Receives zmq socket events and publish them to a PUSH socket
/// with the endpoint associated to the proxy as the topic.
struct MonitorEventProxy {
    map: HashMap<String, RoutingId>,
    server: Server,
    pub_: OldSocket,
}

impl MonitorEventProxy {
    fn new(&mut self) -> Result<(), Error> {
    }

    fn run(&mut self) -> Result<(), Error> {
        loop {
            let mut parts = self.pair.recv_msg_multipart()?;
            let msg = parts.pop().unwrap();

            // Find the client that requested notification for that endpoint.
            if let Some(routing_id) = self.map.get(msg.to_str().unwrap()) {
                let mut msg = parts.pop().unwrap();
                msg.set_routing_id(routing_id);
                self.server.send(msg)?;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{auth::*, prelude::*, socket::*};

    use hashbrown::HashMap;

    use std::{convert::TryInto, thread, time::Duration};

    #[test]
    fn test_null_mechanism() {
        let ctx = Ctx::new();
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::Null)
            .with_ctx(&ctx)
            .unwrap();

        let addr = server.last_endpoint().unwrap().unwrap();

        let client = ClientBuilder::new()
            .connect(addr)
            .mechanism(Mechanism::Null)
            .with_ctx(&ctx)
            .unwrap();

        client.send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_null_plain() {
        fn log(pipe: &mut OldSocket) {
            let mut parts = pipe.recv_msg_multipart().unwrap();
            let event = parts.remove(0);
            let bytes = event.as_bytes();
            let id = u16::from_le_bytes([bytes[0], bytes[1]]);
            let value = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
            dbg!((id, value));
        }

        let ctx = Ctx::new();
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::PlainServer)
            .with_ctx(&ctx)
            .unwrap();

        let inproc: InprocAddr = "server-events".try_into().unwrap();
        monitor_socket(&server, &inproc);

        let mut server_events = OldSocket::with_ctx(OldSocketType::Pair, &ctx).unwrap();
        server_events.connect(inproc).unwrap();

        let addr = server.last_endpoint().unwrap().unwrap();

        //let client = ClientBuilder::new()
        //    .connect(addr)
        //    .mechanism(Mechanism::Null)
        //    .with_ctx(&ctx)
        //    .unwrap();

        let client = Client::with_ctx(&ctx).unwrap();

        let inproc: InprocAddr = "client-events".try_into().unwrap();

        monitor_socket(&client, &inproc);

        let creds = PlainCreds {
            username: "ok".to_owned(),
            password: "lo".to_owned(),
        };
        client.set_mechanism(Mechanism::PlainClient(creds)).unwrap();
        client.connect(&addr).unwrap();

        let mut client_events = OldSocket::with_ctx(OldSocketType::Pair, &ctx).unwrap();
        client_events.connect(inproc).unwrap();

        println!("client");
        for i in 0..3 { log(&mut client_events); }
        println!("server");
        for i in 0..2 { log(&mut server_events); }

        client.send("").unwrap();
        let err = server.try_recv_msg().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::WouldBlock);
    }
}
