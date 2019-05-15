use crate::{*, prelude::*, addr::Endpoint, core::GetRawSocket, old::*};
use libzmq_sys as sys;

use std::{ffi::CString, convert::{TryInto, TryFrom}};

pub struct SocketMonitor {
    sub: OldSocket,
    channel: Server,
}

struct MonitorEventParseError;

pub enum MonitorEvent {
}

impl MonitorEvent {
    fn from_parts(parts: &[Msg]) -> Option<Self> {
        unimplemented!()
    }
}

impl SocketMonitor {
    fn monitor<F>(&mut self, mut f: F) -> Result<(), failure::Error>
    where
        F: FnMut(MonitorEvent) -> Result<(), failure::Error> {
        loop {
            let parts = self.sub.recv_msg_multipart()?;
            let event = MonitorEvent::from_parts(parts.as_slice()).unwrap();
            f(event)?;
        }
    }
}

// Start the SocketLogger only in `log_enabled!(log::INFO)`
// We should decide what events to log based on the logging level.
// https://docs.rs/log/0.4.6/log/
// We should also check if there are any runtime performance cost.
pub(crate) struct SocketLogger {
    inner: SocketMonitor,
}

pub struct SocketLoggerChannel {
    client: Client,
}

impl SocketLoggerChannel {
    fn register<S>(&self, socket: S) -> Result<(), Error> where S: GetRawSocket {
        let endpoint = socket.raw_socket().monitor_addr();
        let msg = Msg::new();
        self.client.send(msg).map_err(Error::into_any)?;
        let msg = self.client.recv_msg()?;
        assert!(msg.is_empty());

        Ok(())
    }

    fn deregister<S>(&self, socket: S) -> Result<(), Error> where S: GetRawSocket {
        let endpoint = socket.raw_socket().monitor_addr();
        let msg = Msg::new();
        self.client.send(msg).map_err(Error::into_any)?;
        let msg = self.client.recv_msg()?;
        assert!(msg.is_empty());

        Ok(())
    }
}

impl SocketMonitor {
}

fn monitor_socket<S, E>(socket: &S, endpoint: E)
where
    S: GetRawSocket,
    E: Into<Endpoint>,
{
    let endpoint = endpoint.into();
    let c_string = CString::new(endpoint.to_zmq()).unwrap();
    let events = sys::ZMQ_EVENT_ALL;

    let rc = unsafe {
        sys::zmq_socket_monitor(
            socket.raw_socket().as_mut_ptr(),
            c_string.as_ptr(),
            events as i32,
        )
    };

    assert_ne!(rc, -1);
}

fn log(pipe: &mut OldSocket) {
    let mut parts = pipe.recv_msg_multipart().unwrap();
    let event = parts.remove(0);
    let bytes = event.as_bytes();
    let id = u16::from_le_bytes([bytes[0], bytes[1]]);
    let value =
        u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    dbg!((id, value));
}
