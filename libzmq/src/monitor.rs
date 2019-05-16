use crate::{
    *, socket::*
    addr::Endpoint,
    auth::StatusCode,
    core::{
        sockopt::{setsockopt_str, SocketOption},
        GetRawSocket,
    },
    old::*,
    prelude::*,
    *,
};
use libzmq_sys as sys;

use failure::Fail;
use libc::c_long;
use log::{error, info};

use std::{
    convert::{TryFrom, TryInto},
    ffi::CString,
    time::Duration,
};

#[derive(Debug, Fail)]
#[fail(display = "unable to parse event")]
struct EventCodeParseError(());

enum EventCode {
    Connected,               // ZMQ_EVENT_CONNECTED
    ConnectDelayed,          // ZMQ_EVENT_CONNECT_DELAYED
    ConnectRetried,          // ZMQ_EVENT_CONNECT_RETRIED
    Bound,                   // ZMQ_EVENT_LISTENING
    BindFailed,              // ZMQ_EVENT_BIND_FAILED
    Accepted,                // ZMQ_EVENT_ACCEPTED
    AcceptFailed,            // ZMQ_EVENT_ACCEPT_FAILED
    Closed,                  // ZMQ_EVENT_CLOSED
    CloseFailed,             // ZMQ_EVENT_CLOSE_FAILED
    Disconnected,            // ZMQ_EVENT_DISCONNECTED
    Stopped,                 // ZMQ_EVENT_MONITOR_STOPPED
    HandshakeFailedNoDetail, // ZMQ_EVENT_HANDSHAKE_FAILED_NO_DETAIL
    HandshakeSucceeded,      // ZMQ_EVENT_HANDSHAKE_SUCCEEDED
    HandshakeFailedProtocol, // ZMQ_EVENT_HANDSHAKE_FAILED_PROTOCOL
    HandshakeFailedAuth,     // ZMQ_EVENT_HANDSHAKE_FAILED_AUTH
}

impl<'a> TryFrom<i64> for EventCode {
    type Error = EventCodeParseError;
    fn try_from(x: i64) -> Result<Self, EventCodeParseError> {
        match x {
            x if x == sys::ZMQ_EVENT_CONNECTED as c_long => {
                Ok(EventCode::Connected)
            }
            x if x == sys::ZMQ_EVENT_CONNECT_DELAYED as c_long => {
                Ok(EventCode::ConnectDelayed)
            }
            x if x == sys::ZMQ_EVENT_CONNECT_RETRIED as c_long => {
                Ok(EventCode::ConnectRetried)
            }
            x if x == sys::ZMQ_EVENT_LISTENING as c_long => {
                Ok(EventCode::Bound)
            }
            x if x == sys::ZMQ_EVENT_BIND_FAILED as c_long => {
                Ok(EventCode::BindFailed)
            }
            x if x == sys::ZMQ_EVENT_ACCEPTED as c_long => {
                Ok(EventCode::Accepted)
            }
            x if x == sys::ZMQ_EVENT_ACCEPT_FAILED as c_long => {
                Ok(EventCode::AcceptFailed)
            }
            x if x == sys::ZMQ_EVENT_CLOSED as c_long => Ok(EventCode::Closed),
            x if x == sys::ZMQ_EVENT_CLOSE_FAILED as c_long => {
                Ok(EventCode::CloseFailed)
            }
            x if x == sys::ZMQ_EVENT_DISCONNECTED as c_long => {
                Ok(EventCode::Disconnected)
            }
            x if x == sys::ZMQ_EVENT_MONITOR_STOPPED as c_long => {
                Ok(EventCode::Stopped)
            }
            x if x == sys::ZMQ_EVENT_HANDSHAKE_FAILED_NO_DETAIL as c_long => {
                Ok(EventCode::HandshakeFailedNoDetail)
            }
            x if x == sys::ZMQ_EVENT_HANDSHAKE_SUCCEEDED as c_long => {
                Ok(EventCode::HandshakeSucceeded)
            }
            x if x == sys::ZMQ_EVENT_HANDSHAKE_FAILED_PROTOCOL as c_long => {
                Ok(EventCode::HandshakeFailedProtocol)
            }
            x if x == sys::ZMQ_EVENT_HANDSHAKE_FAILED_AUTH as c_long => {
                Ok(EventCode::HandshakeFailedAuth)
            }
            _ => Err(EventCodeParseError(())),
        }
    }
}

impl<'a> TryFrom<&'a str> for EventCode {
    type Error = EventCodeParseError;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let code: i64 = s.parse().map_err(|_| EventCodeParseError(()))?;
        Self::try_from(code)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum HandshakeError {}

#[derive(Debug, Copy, Clone)]
pub enum EventType {
    Connected,
    ConnectDelayed,
    Bound,
    Accepted,
    Closed,
    Disconnected,
    Stopped,
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
    fn event_type(&self) -> EventType {
        self.event_type
    }

    fn local_endpoint(&self) -> Option<&Endpoint> {
        self.local_endpoint.as_ref()
    }

    fn remote_endpoint(&self) -> Option<&Endpoint> {
        self.remote_endpoint.as_ref()
    }
}

pub struct SocketMonitor {
    sub: OldSocket,
    channel: Server,
    addr: InprocAddr,
}

// * The first frame contains an event number (64 bits)
// * Second frame contains the number of value frames that will follow it as
//   a 64 bits integer
// * The third frame to N-th frames contain an event value (64 bits) that
//   provides additional data according to the event number.
// * The second-to-last and last frames contain strings that specifies the
//   affected connection or endpoint. The former frame contains a string
//   denoting the local endpoint, while the latter frame contains a string
//   denoting the remote endpoint. Either of these may be empty, depending on
//   the event type and whether the connection uses a bound or connected local
//   endpoint.
impl SocketMonitor {
    fn new() -> Result<Self, Error> {
        let sub = OldSocket::new(OldSocketType::Sub)?;
        let addr = InprocAddr::new_unique();
        let channel = ServerBuilder::new()
            .bind(&addr)
            .build()?;

        Ok(Self {
            sub,
            addr,
            channel,
        })
    }

    fn monitor<F>(&mut self, mut f: F) -> Result<(), failure::Error>
    where
        F: FnMut(MonitorEvent) -> Result<(), failure::Error>,
    {
        loop {
            let event = self.next_event()?;
            f(event)?;
        }
    }

    fn subscribe(&mut self, topic: EventCode) -> Result<(), Error> {
        let topic = (topic as u64).to_string();
        setsockopt_str(
            self.sub.raw_socket().as_mut_ptr(),
            SocketOption::Subscribe,
            Some(&topic),
        )
    }

    fn unsubscribe(&mut self, topic: EventCode) -> Result<(), Error> {
        let topic = (topic as u64).to_string();
        setsockopt_str(
            self.sub.raw_socket().as_mut_ptr(),
            SocketOption::Unsubscribe,
            Some(&topic),
        )
    }

    fn next_event(&mut self) -> Result<MonitorEvent, Error> {
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

        let code: EventCode =
            parts.remove(0).to_str().unwrap().try_into().unwrap();
        parts.remove(0);

        let event_type = {
            match code {
                EventCode::Connected => EventType::Connected,
                EventCode::ConnectDelayed => EventType::ConnectDelayed,
                EventCode::Bound => EventType::Bound,
                EventCode::Accepted => EventType::Accepted,
                EventCode::Closed => EventType::Closed,
                EventCode::Disconnected => EventType::Disconnected,
                EventCode::Stopped => EventType::Stopped,
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
                        parts.remove(0).to_str().unwrap().try_into().unwrap();
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
    fn run(&mut self) -> Result<(), failure::Error> {
        let log = |event| {
            Self::log(event)
        };
        self.inner.monitor(log)
    }

    fn log(event: MonitorEvent) -> Result<(), failure::Error> {
        use EventType::*;
        match event.event_type() {
            Connected | ConnectDelayed | Bound | Accepted | Closed
            | Disconnected | Stopped | HandshakeSucceeded
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

        Ok(())
    }
}

impl SocketLogger {}

pub struct SocketLoggerChannel {
    client: Client,
}

impl SocketLoggerChannel {
    fn register<S>(&self, socket: S) -> Result<(), Error>
    where
        S: GetRawSocket,
    {
        let endpoint = socket.raw_socket().monitor_addr();
        let msg = Msg::new();
        self.client.send(msg).map_err(Error::into_any)?;
        let msg = self.client.recv_msg()?;
        assert!(msg.is_empty());

        Ok(())
    }

    fn deregister<S>(&self, socket: S) -> Result<(), Error>
    where
        S: GetRawSocket,
    {
        let endpoint = socket.raw_socket().monitor_addr();
        let msg = Msg::new();
        self.client.send(msg).map_err(Error::into_any)?;
        let msg = self.client.recv_msg()?;
        assert!(msg.is_empty());

        Ok(())
    }
}

// Create a PUB socket for monitoring with all the possible events specified.
pub(crate) fn init_socket_monitor<S, E>(socket: &S, endpoint: E)
where
    S: GetRawSocket,
    E: Into<Endpoint>,
{
    let endpoint = endpoint.into();
    let c_string = CString::new(endpoint.to_zmq()).unwrap();

    let rc = unsafe {
        sys::zmq_socket_monitor_versioned_typed(
            socket.raw_socket().as_mut_ptr(),
            c_string.as_ptr(),
            sys::ZMQ_EVENT_ALL_V2 as u64,
            2,
            sys::ZMQ_PUB as i32,
        )
    };

    assert_ne!(rc, -1);
}
