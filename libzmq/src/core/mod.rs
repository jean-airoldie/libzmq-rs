//! The set of core ØMQ socket traits.

mod raw;
mod recv;
mod send;
pub(crate) mod sockopt;

pub(crate) use raw::{GetRawSocket, RawSocket, RawSocketType};

pub use recv::*;
pub use send::*;

/// Prevent users from implementing the AsRawSocket trait.
mod private {
    use super::*;
    use crate::socket::*;

    pub trait Sealed {}
    impl Sealed for SocketConfig {}
    impl Sealed for SendConfig {}
    impl Sealed for RecvConfig {}
    impl Sealed for Client {}
    impl Sealed for ClientConfig {}
    impl Sealed for ClientBuilder {}
    impl Sealed for Server {}
    impl Sealed for ServerConfig {}
    impl Sealed for ServerBuilder {}
    impl Sealed for Radio {}
    impl Sealed for RadioConfig {}
    impl Sealed for RadioBuilder {}
    impl Sealed for Dish {}
    impl Sealed for DishConfig {}
    impl Sealed for DishBuilder {}
}

use crate::{
    endpoint::Endpoint,
    error::{msg_from_errno, Error, ErrorKind},
};
use libzmq_sys as sys;
use sockopt::*;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{ffi::CString, os::raw::c_void, time::Duration};

const MAX_HB_TTL: i64 = 6_553_599;

fn connect(mut_raw_socket: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_connect(mut_raw_socket, c_str.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                    msg: "invalid endpoint",
                }),
                errno::EPROTONOSUPPORT => Error::new(ErrorKind::InvalidInput {
                    msg: "endpoint protocol not supported",
                }),
                errno::ENOCOMPATPROTO => Error::new(ErrorKind::InvalidInput {
                    msg: "endpoint protocol incompatible",
                }),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EMTHREAD => panic!("no i/o thread available"),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn bind(mut_raw_socket: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_bind(mut_raw_socket, c_str.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                    msg: "invalid endpoint",
                }),
                errno::EPROTONOSUPPORT => Error::new(ErrorKind::InvalidInput {
                    msg: "endpoint protocol not supported",
                }),
                errno::ENOCOMPATPROTO => Error::new(ErrorKind::InvalidInput {
                    msg: "endpoint protocol incompatible",
                }),
                errno::EADDRINUSE => Error::new(ErrorKind::AddrInUse),
                errno::EADDRNOTAVAIL => Error::new(ErrorKind::AddrNotAvailable),
                errno::ENODEV => Error::new(ErrorKind::AddrNotAvailable),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EMTHREAD => panic!("no i/o thread available"),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn disconnect(
    mut_raw_socket: *mut c_void,
    c_str: CString,
) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_disconnect(mut_raw_socket, c_str.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                    msg: "invalid endpoint",
                }),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::ENOENT => Error::new(ErrorKind::NotFound {
                    msg: "endpoint was not connected to",
                }),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn unbind(mut_raw_socket: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_unbind(mut_raw_socket, c_str.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                    msg: "invalid endpoint",
                }),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::ENOENT => Error::new(ErrorKind::NotFound {
                    msg: "endpoint was not bound to",
                }),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

/// Methods shared by all thread-safe sockets.
pub trait Socket: GetRawSocket {
    /// Connects the socket to an [`endpoint`] and then accepts incoming connections
    /// on that [`endpoint`].
    ///
    /// The socket actually connects a few instants after the `connect` call
    /// (usually less than a millisecond).
    ///
    /// See [`zmq_connect`].
    ///
    /// # Usage Contract
    /// TODO
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if contract not followed)
    /// * [`IncompatTransport`]
    /// * [`CtxTerminated`]
    ///
    /// [`endpoint`]: #endpoint
    /// [`zmq_connect`]: http://api.zeromq.org/master:zmq-connect
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`IncompatTransport`]: ../enum.ErrorKind.html#variant.IncompatTransport
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    fn connect<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: AsRef<Endpoint>,
    {
        let endpoint = endpoint.as_ref();
        let c_str = CString::new(endpoint.to_string()).unwrap();
        connect(self.mut_raw_socket(), c_str)
    }

    /// Disconnect the socket from the endpoint.
    ///
    /// Any outstanding messages physically received from the network but not
    /// yet received by the application are discarded. The behaviour for
    /// discarding messages depends on the value of [`linger`].
    ///
    /// See [`zmq_disconnect`].
    ///
    /// # Usage Contract
    /// TODO
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if contract not followed)
    /// * [`CtxTerminated`]
    /// * [`NotFound`] (if endpoint not connected to)
    ///
    /// [`zmq_disconnect`]: http://api.zeromq.org/master:zmq-disconnect
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    /// [`linger`]: #method.linger
    fn disconnect<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: AsRef<Endpoint>,
    {
        let endpoint = endpoint.as_ref();
        let c_str = CString::new(endpoint.to_string()).unwrap();
        disconnect(self.mut_raw_socket(), c_str)
    }

    /// Binds the socket to a local [`endpoint`] and then accepts incoming
    /// connections.
    ///
    /// The socket actually binds a few instants after the `bind` call
    /// (usually less than a millisecond).
    ///
    /// See [`zmq_bind`].
    ///
    /// # Usage Contract
    /// TODO
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if usage contract not followed)
    /// * [`IncompatTransport`]
    /// * [`AddrInUse`]
    /// * [`AddrNotAvailable`]
    /// * [`CtxTerminated`]
    ///
    /// [`endpoint`]: #endpoint
    /// [`zmq_bind`]: http://api.zeromq.org/master:zmq-bind
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`IncompatTransport`]: ../enum.ErrorKind.html#variant.IncompatTransport
    /// [`AddrInUse`]: ../enum.ErrorKind.html#variant.AddrInUse
    /// [`AddrNotAvailable`]: ../enum.ErrorKind.html#variant.AddrNotAvailable
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    fn bind<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: AsRef<Endpoint>,
    {
        let endpoint = endpoint.as_ref();
        let c_str = CString::new(endpoint.to_string()).unwrap();
        bind(self.mut_raw_socket(), c_str)
    }

    /// Unbinds the socket from the endpoint.
    ///
    /// Any outstanding messages physically received from the network but not
    /// yet received by the application are discarded. The behaviour for
    /// discarding messages depends on the value of [`linger`].
    ///
    /// See [`zmq_unbind`].
    ///
    /// # Usage Contract
    /// TODO
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if usage contract not followed)
    /// * [`CtxTerminated`]
    /// * [`NotFound`] (if endpoint was not bound to)
    ///
    /// [`zmq_unbind`]: http://api.zeromq.org/master:zmq-unbind
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    /// [`linger`]: #method.linger
    fn unbind<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: AsRef<Endpoint>,
    {
        let endpoint = endpoint.as_ref();
        let c_str = CString::new(endpoint.to_string()).unwrap();
        unbind(self.mut_raw_socket(), c_str)
    }

    /// Retrieve the maximum length of the queue of outstanding peer connections.
    ///
    /// See `ZMQ_BLACKLOG` in [`zmq_getsockopt`].
    ///
    /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
    fn backlog(&self) -> Result<i32, Error> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_scalar(mut_raw_socket, SocketOption::Backlog)
    }

    /// Set the maximum length of the queue of outstanding peer connections
    /// for the specified socket; this only applies to connection-oriented
    /// transports.
    ///
    /// See `ZMQ_BACKLOG` in [`zmq_setsockopt`].
    ///
    /// # Default Value
    /// 100
    ///
    /// # Applicable Socket Type
    /// All (Connection Oriented Transports)
    ///
    /// [`zmq_setsockopt`]: http://api.zeromq.org/master:zmq-setsockopt
    fn set_backlog(&self, value: i32) -> Result<(), Error> {
        setsockopt_scalar(self.mut_raw_socket(), SocketOption::Backlog, value)
    }

    /// Retrieves how many milliseconds to wait before timing-out a [`connect`]
    /// call.
    ///
    /// See `ZMQ_CONNECT_TIMEOUT` in [`zmq_getsockopt`].
    ///
    /// [`connect`]: #method.connect
    /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
    fn connect_timeout(&self) -> Result<Option<Duration>, Error> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::ConnectTimeout, -1)
    }

    /// Sets how much time to wait before timing-out a [`connect`] call.
    ///
    /// The `connect` call normally takes a long time before it returns
    /// a time out error.
    ///
    /// # Default Value
    /// `None`
    ///
    /// # Applicable Socket Type
    /// All (TCP transport)
    fn set_connect_timeout(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error> {
        if let Some(ref duration) = maybe_duration {
            assert!(
                duration.as_millis() > 0,
                "number of ms in duration cannot be zero"
            );
        }
        // This is safe the call does not actually mutate the socket.
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::ConnectTimeout,
            maybe_duration,
            0,
        )
    }

    /// The interval between sending ZMTP heartbeats.
    fn heartbeat_interval(&self) -> Result<Option<Duration>, Error> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::HeartbeatInterval, 0)
    }

    /// Sets the interval between sending ZMTP PINGs (aka. heartbeats).
    ///
    /// # Default Value
    /// `None`
    ///
    /// # Applicable Socket Type
    /// All (connection oriented transports)
    fn set_heartbeat_interval(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error> {
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::HeartbeatInterval,
            maybe_duration,
            0,
        )
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    fn heartbeat_timeout(&self) -> Result<Option<Duration>, Error> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::HeartbeatTimeout, 0)
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    ///
    /// # Default Value
    /// `None`. If `heartbeat_interval` is set, then it uses the same value
    /// by default.
    fn set_heartbeat_timeout(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error> {
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::HeartbeatTimeout,
            maybe_duration,
            0,
        )
    }

    /// The timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    fn heartbeat_ttl(&self) -> Result<Option<Duration>, Error> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::HeartbeatTtl, 0)
    }

    /// Set timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    ///
    /// # Default value
    /// `None`
    fn set_heartbeat_ttl(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error> {
        if let Some(ref duration) = maybe_duration {
            let ms = duration.as_millis();
            if ms > MAX_HB_TTL as u128 {
                return Err(Error::new(ErrorKind::InvalidInput {
                    msg: "duration ms cannot exceed 6553599",
                }));
            }
        }
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::HeartbeatTtl,
            maybe_duration,
            0,
        )
    }

    /// Returns the linger period for the socket shutdown.
    fn linger(&self) -> Result<Option<Duration>, Error> {
        // This is safe since the call does not actually mutate the socket.
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::Linger, -1)
    }

    /// Sets the linger period for the socket shutdown.
    ///
    /// The linger period determines how long pending messages which have
    /// yet to be sent to a peer shall linger in memory after a socket is
    /// disconnected or dropped.
    ///
    /// A value of `None` means an infinite period.
    ///
    /// # Default Value
    /// 30 secs
    fn set_linger(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error> {
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::Linger,
            maybe_duration,
            -1,
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[doc(hidden)]
pub struct SocketConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    backlog: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    connect_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_interval: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_ttl: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    linger: Option<Duration>,
}

impl SocketConfig {
    pub(crate) fn apply<S: Socket>(&self, socket: &S) -> Result<(), Error> {
        if let Some(value) = self.backlog {
            socket.set_backlog(value)?;
        }
        socket.set_connect_timeout(self.connect_timeout)?;
        socket.set_heartbeat_interval(self.heartbeat_interval)?;
        socket.set_heartbeat_timeout(self.heartbeat_timeout)?;
        socket.set_heartbeat_ttl(self.heartbeat_ttl)?;
        socket.set_linger(self.linger)?;

        // We connect as the last step because some socket options
        // only affect subsequent connections.
        if let Some(ref endpoints) = self.connect {
            for endpoint in endpoints {
                socket.connect(endpoint)?;
            }
        }
        if let Some(ref endpoints) = self.bind {
            for endpoint in endpoints {
                socket.bind(endpoint)?;
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub trait GetSocketConfig: private::Sealed {
    fn socket_config(&self) -> &SocketConfig;

    fn mut_socket_config(&mut self) -> &mut SocketConfig;
}

impl GetSocketConfig for SocketConfig {
    fn socket_config(&self) -> &SocketConfig {
        self
    }

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        self
    }
}

pub trait ConfigureSocket: GetSocketConfig {
    fn connect(&self) -> Option<&[Endpoint]> {
        self.socket_config().connect.as_ref().map(Vec::as_slice)
    }

    fn set_connect<E>(&mut self, maybe_endpoints: Option<E>)
    where
        E: IntoIterator<Item = Endpoint>,
    {
        let maybe_vec: Option<Vec<Endpoint>> =
            maybe_endpoints.map(|e| e.into_iter().collect());
        self.mut_socket_config().connect = maybe_vec;
    }

    fn bind(&self) -> Option<&[Endpoint]> {
        self.socket_config().bind.as_ref().map(Vec::as_slice)
    }

    fn set_bind<E>(&mut self, maybe_endpoints: Option<E>)
    where
        E: IntoIterator<Item = Endpoint>,
    {
        let maybe_vec: Option<Vec<Endpoint>> =
            maybe_endpoints.map(|e| e.into_iter().collect());
        self.mut_socket_config().bind = maybe_vec;
    }

    fn backlog(&self) -> Option<i32> {
        self.socket_config().backlog
    }

    fn set_backlog(&mut self, maybe_backlog: Option<i32>) {
        self.mut_socket_config().backlog = maybe_backlog;
    }

    fn connect_timeout(&self) -> Option<Duration> {
        self.socket_config().connect_timeout
    }

    fn set_connect_timeout(&mut self, maybe_duration: Option<Duration>) {
        self.mut_socket_config().connect_timeout = maybe_duration;
    }

    fn heartbeat_interval(&self) -> Option<Duration> {
        self.socket_config().heartbeat_interval
    }

    fn set_heartbeat_interval(&mut self, maybe_duration: Option<Duration>) {
        self.mut_socket_config().heartbeat_interval = maybe_duration;
    }

    fn heartbeat_timeout(&self) -> Option<Duration> {
        self.socket_config().heartbeat_timeout
    }

    fn set_heartbeat_timeout(&mut self, maybe_duration: Option<Duration>) {
        self.mut_socket_config().heartbeat_timeout = maybe_duration;
    }

    fn heartbeat_ttl(&self) -> Option<Duration> {
        self.socket_config().heartbeat_ttl
    }

    fn set_heartbeat_ttl(&mut self, maybe_duration: Option<Duration>) {
        self.mut_socket_config().heartbeat_ttl = maybe_duration;
    }

    fn linger(&self) -> Option<Duration> {
        self.socket_config().linger
    }

    fn set_linger(&mut self, maybe_duration: Option<Duration>) {
        self.mut_socket_config().linger = maybe_duration;
    }
}

impl ConfigureSocket for SocketConfig {}

pub trait BuildSocket: GetSocketConfig + Sized {
    fn connect<E>(&mut self, endpoints: E) -> &mut Self
    where
        E: IntoIterator<Item = Endpoint>,
    {
        self.mut_socket_config().set_connect(Some(endpoints));
        self
    }

    fn bind<E>(&mut self, endpoints: E) -> &mut Self
    where
        E: IntoIterator<Item = Endpoint>,
    {
        self.mut_socket_config().set_bind(Some(endpoints));
        self
    }

    fn backlog(&mut self, len: i32) -> &mut Self {
        self.mut_socket_config().set_backlog(Some(len));
        self
    }

    fn connect_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        self.mut_socket_config().set_connect_timeout(maybe_duration);
        self
    }

    fn heartbeat_interval(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        self.mut_socket_config()
            .set_heartbeat_interval(maybe_duration);
        self
    }

    fn heartbeat_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        self.mut_socket_config()
            .set_heartbeat_timeout(maybe_duration);
        self
    }

    fn heartbeat_ttl(&mut self, maybe_duration: Option<Duration>) -> &mut Self {
        self.mut_socket_config().set_heartbeat_ttl(maybe_duration);
        self
    }

    fn linger(&mut self, maybe_duration: Option<Duration>) -> &mut Self {
        self.mut_socket_config().set_linger(maybe_duration);
        self
    }
}
