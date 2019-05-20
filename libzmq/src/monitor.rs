//! Asynchronous socket event monitoring.

use crate::{addr::Endpoint, auth::StatusCode, core::GetRawSocket, old::*, *};
use libzmq_sys as sys;

use failure::Fail;
use libc::c_void;
use log::{error, info};

use std::{
    convert::{TryFrom, TryInto},
    ffi::CString,
    time::Duration,
};

#[doc(hidden)]
#[derive(Debug, Fail)]
#[fail(display = "unable to parse event")]
pub struct EventCodeParseError(());

pub enum EventCode {
    Connected = sys::ZMQ_EVENT_CONNECTED as isize,
    ConnectDelayed = sys::ZMQ_EVENT_CONNECT_DELAYED as isize,
    ConnectRetried = sys::ZMQ_EVENT_CONNECT_RETRIED as isize,
    Bound = sys::ZMQ_EVENT_LISTENING as isize,
    BindFailed = sys::ZMQ_EVENT_BIND_FAILED as isize,
    Accepted = sys::ZMQ_EVENT_ACCEPTED as isize,
    AcceptFailed = sys::ZMQ_EVENT_ACCEPT_FAILED as isize,
    Closed = sys::ZMQ_EVENT_CLOSED as isize,
    CloseFailed = sys::ZMQ_EVENT_CLOSE_FAILED as isize,
    Disconnected = sys::ZMQ_EVENT_DISCONNECTED as isize,
    MonitorStopped = sys::ZMQ_EVENT_MONITOR_STOPPED as isize,
    HandshakeFailedNoDetail =
        sys::ZMQ_EVENT_HANDSHAKE_FAILED_NO_DETAIL as isize,
    HandshakeSucceeded = sys::ZMQ_EVENT_HANDSHAKE_SUCCEEDED as isize,
    HandshakeFailedProtocol = sys::ZMQ_EVENT_HANDSHAKE_FAILED_PROTOCOL as isize,
    HandshakeFailedAuth = sys::ZMQ_EVENT_HANDSHAKE_FAILED_AUTH as isize,
}

#[doc(hidden)]
impl<'a> TryFrom<u64> for EventCode {
    type Error = EventCodeParseError;
    fn try_from(x: u64) -> Result<Self, EventCodeParseError> {
        match x {
            x if x == EventCode::Connected as u64 => Ok(EventCode::Connected),
            x if x == EventCode::ConnectDelayed as u64 => {
                Ok(EventCode::ConnectDelayed)
            }
            x if x == EventCode::ConnectRetried as u64 => {
                Ok(EventCode::ConnectRetried)
            }
            x if x == EventCode::Bound as u64 => Ok(EventCode::Bound),
            x if x == EventCode::BindFailed as u64 => Ok(EventCode::BindFailed),
            x if x == EventCode::Accepted as u64 => Ok(EventCode::Accepted),
            x if x == EventCode::AcceptFailed as u64 => {
                Ok(EventCode::AcceptFailed)
            }
            x if x == EventCode::Closed as u64 => Ok(EventCode::Closed),
            x if x == EventCode::CloseFailed as u64 => {
                Ok(EventCode::CloseFailed)
            }
            x if x == EventCode::Disconnected as u64 => {
                Ok(EventCode::Disconnected)
            }
            x if x == EventCode::MonitorStopped as u64 => {
                Ok(EventCode::MonitorStopped)
            }
            x if x == EventCode::HandshakeFailedNoDetail as u64 => {
                Ok(EventCode::HandshakeFailedNoDetail)
            }
            x if x == EventCode::HandshakeSucceeded as u64 => {
                Ok(EventCode::HandshakeSucceeded)
            }
            x if x == EventCode::HandshakeFailedProtocol as u64 => {
                Ok(EventCode::HandshakeFailedProtocol)
            }
            x if x == EventCode::HandshakeFailedAuth as u64 => {
                Ok(EventCode::HandshakeFailedAuth)
            }
            _ => Err(EventCodeParseError(())),
        }
    }
}

#[doc(hidden)]
impl<'a> TryFrom<&'a [u8]> for EventCode {
    type Error = EventCodeParseError;
    fn try_from(a: &'a [u8]) -> Result<Self, Self::Error> {
        let mut bytes: [u8; 8] = Default::default();
        bytes.copy_from_slice(a);
        let code = dbg!(u64::from_ne_bytes(bytes));
        Self::try_from(code)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HandshakeError {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EventType {
    Connected,
    ConnectDelayed,
    Bound,
    Accepted,
    Closed,
    Disconnected,
    MonitorStopped,
    HandshakeSucceeded,
    ConnectRetried(Duration),
    BindFailed,
    AcceptFailed,
    CloseFailed,
    HandshakeFailedNoDetail,
    HandshakeFailedProtocol(HandshakeError),
    HandshakeFailedAuth(StatusCode),
}

#[derive(Debug, Clone)]
pub struct MonitorEvent {
    event_type: EventType,
    local_endpoint: Option<Endpoint>,
    remote_endpoint: Option<Endpoint>,
}

impl MonitorEvent {
    pub fn event_type(&self) -> EventType {
        self.event_type
    }

    pub fn local_endpoint(&self) -> Option<&Endpoint> {
        self.local_endpoint.as_ref()
    }

    pub fn remote_endpoint(&self) -> Option<&Endpoint> {
        self.remote_endpoint.as_ref()
    }
}

pub struct SocketMonitor {
    sub: OldSocket,
}

impl SocketMonitor {
    pub fn new() -> Result<Self, Error> {
        let sub = OldSocket::new(OldSocketType::Sub)?;

        Ok(Self { sub })
    }

    pub fn register<S>(&mut self, socket: &S) -> Result<(), Error>
    where
        S: GetRawSocket,
    {
        let addr = socket.raw_socket().monitor_addr();
        self.sub.connect(addr)
    }

    pub fn deregister<S>(&mut self, socket: &S) -> Result<(), Error>
    where
        S: GetRawSocket,
    {
        let addr = socket.raw_socket().monitor_addr();
        self.sub.disconnect(addr)
    }

    pub(crate) fn subscribe_all(&mut self) -> Result<(), Error> {
        self.sub.subscribe(b"")
    }

    pub fn subscribe(&mut self, topic: EventCode) -> Result<(), Error> {
        let topic = (topic as u64).to_ne_bytes();
        self.sub.subscribe(&topic)
    }

    pub fn unsubscribe(&mut self, topic: EventCode) -> Result<(), Error> {
        let topic = (topic as u64).to_ne_bytes();
        self.sub.unsubscribe(&topic)
    }

    pub fn set_recv_timeout(
        &mut self,
        maybe: Option<Duration>,
    ) -> Result<(), Error> {
        self.sub.set_recv_timeout(maybe)
    }

    pub fn recv_timeout(&self) -> Result<Option<Duration>, Error> {
        self.sub.recv_timeout()
    }

    pub fn recv_event(&mut self) -> Result<MonitorEvent, Error> {
        let mut parts = self.sub.recv_msg_multipart()?;

        let remote_endpoint = {
            let msg = parts.pop().unwrap();
            if msg.is_empty() {
                None
            } else {
                Some(Endpoint::from_zmq(msg.to_str().unwrap()))
            }
        };
        let local_endpoint = {
            let msg = parts.pop().unwrap();
            if msg.is_empty() {
                None
            } else {
                Some(Endpoint::from_zmq(msg.to_str().unwrap()))
            }
        };

        let code: EventCode = parts.remove(0).as_bytes().try_into().unwrap();
        parts.remove(0);

        let event_type = {
            match code {
                EventCode::Connected => EventType::Connected,
                EventCode::ConnectDelayed => EventType::ConnectDelayed,
                EventCode::Bound => EventType::Bound,
                EventCode::Accepted => EventType::Accepted,
                EventCode::Closed => EventType::Closed,
                EventCode::Disconnected => EventType::Disconnected,
                EventCode::MonitorStopped => EventType::MonitorStopped,
                EventCode::HandshakeSucceeded => EventType::HandshakeSucceeded,
                EventCode::ConnectRetried => {
                    let ms: u64 =
                        parts.remove(0).to_str().unwrap().parse().unwrap();
                    let duration = Duration::from_millis(ms);
                    EventType::ConnectRetried(duration)
                }
                EventCode::BindFailed => EventType::BindFailed,
                EventCode::AcceptFailed => EventType::AcceptFailed,
                EventCode::CloseFailed => EventType::CloseFailed,
                EventCode::HandshakeFailedNoDetail => {
                    EventType::HandshakeFailedNoDetail
                }
                EventCode::HandshakeFailedProtocol => unimplemented!(),
                EventCode::HandshakeFailedAuth => {
                    let status: StatusCode =
                        parts.remove(0).as_bytes().try_into().unwrap();
                    EventType::HandshakeFailedAuth(status)
                }
            }
        };

        Ok(MonitorEvent {
            event_type,
            local_endpoint,
            remote_endpoint,
        })
    }
}

// Start the SocketLogger only in `log_enabled!(log::INFO)`
// We should decide what events to log based on the logging level.
// https://docs.rs/log/0.4.6/log/
// We should also check if there are any runtime performance cost.
pub(crate) struct SocketLogger {
    inner: SocketMonitor,
}

impl SocketLogger {
    pub(crate) fn new() -> Result<Self, Error> {
        let mut inner = SocketMonitor::new()?;

        inner.subscribe_all()?;

        Ok(Self { inner })
    }

    pub(crate) fn run(&mut self) -> Result<(), failure::Error> {
        loop {
            let event = self.inner.recv_event()?;
            Self::log(event);
        }
    }

    fn log(event: MonitorEvent) {
        use EventType::*;
        match event.event_type() {
            Connected | ConnectDelayed | Bound | Accepted | Closed
            | Disconnected | MonitorStopped | HandshakeSucceeded
            | ConnectRetried(_) => {
                info!("{:?}", event);
            }
            BindFailed
            | AcceptFailed
            | CloseFailed
            | HandshakeFailedNoDetail
            | HandshakeFailedProtocol(_)
            | HandshakeFailedAuth(_) => {
                error!("{:?}", event);
            }
        }
    }
}

impl SocketLogger {}

// Create a PUB socket that monitors all socket events.
pub(crate) fn init_socket_monitor<E>(socket_mut_ptr: *mut c_void, endpoint: E)
where
    E: Into<Endpoint>,
{
    let endpoint = endpoint.into();
    let c_string = CString::new(endpoint.to_zmq()).unwrap();

    let rc = unsafe {
        sys::zmq_socket_monitor_versioned(
            socket_mut_ptr,
            c_string.as_ptr(),
            u64::from(sys::ZMQ_EVENT_ALL_V2),
            2,
            sys::ZMQ_PUB as i32,
        )
    };

    assert_ne!(rc, -1);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{prelude::*, socket::*};

    fn expect_event(monitor: &mut SocketMonitor, expected: EventType) {
        let event = dbg!(monitor.recv_event().unwrap());
        assert_eq!(event.event_type(), expected);
    }

    #[test]
    fn test_socket_monitor() {
        let mut client_monitor = SocketMonitor::new().unwrap();
        client_monitor.subscribe_all().unwrap();

        let mut server_monitor = SocketMonitor::new().unwrap();
        server_monitor.subscribe_all().unwrap();
        {
            let client = Client::new().unwrap();
            let server = Server::new().unwrap();

            // Register both sockets for monitoring.
            client_monitor.register(&client).unwrap();
            server_monitor.register(&server).unwrap();

            let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
            server.bind(addr).unwrap();

            expect_event(&mut server_monitor, EventType::Bound);

            let addr = server.last_endpoint().unwrap().unwrap();
            client.connect(addr).unwrap();

            // The order is random.
            expect_event(&mut server_monitor, EventType::Accepted);
            expect_event(&mut server_monitor, EventType::HandshakeSucceeded);

            expect_event(&mut client_monitor, EventType::ConnectDelayed);
            expect_event(&mut client_monitor, EventType::Connected);
            expect_event(&mut client_monitor, EventType::HandshakeSucceeded);
        }
    }

    #[test]
    fn test_socket_monitor_subscribe() {
        let mut monitor = SocketMonitor::new().unwrap();
        monitor.subscribe(EventCode::Closed).unwrap();

        {
            let server = Server::new().unwrap();
            monitor.register(&server).unwrap();

            // Let some time for the socket monitor to start and start pumping events.
            let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
            server.bind(addr).unwrap();
        }

        expect_event(&mut monitor, EventType::Closed);
    }
}
