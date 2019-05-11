//! The set of core ØMQ socket traits.

mod raw;
mod recv;
mod send;
pub(crate) mod sockopt;

pub use raw::AuthRole;
pub(crate) use raw::*;

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
    impl Sealed for SocketType {}

    // Pub crate
    use crate::auth::Dealer;
    impl Sealed for Dealer {}
}

use crate::{
    addr::Endpoint,
    auth::{Creds, Mechanism},
    error::{msg_from_errno, Error, ErrorKind},
};
use libzmq_sys as sys;
use sockopt::*;
use sys::errno;

use std::{ffi::CString, os::raw::c_void, time::Duration};

const MAX_HB_TTL: i64 = 6_553_599;

fn connect(socket_ptr: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_connect(socket_ptr, c_str.as_ptr()) };

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

fn bind(socket_ptr: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_bind(socket_ptr, c_str.as_ptr()) };

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

fn disconnect(socket_ptr: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_disconnect(socket_ptr, c_str.as_ptr()) };

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

fn unbind(socket_ptr: *mut c_void, c_str: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_unbind(socket_ptr, c_str.as_ptr()) };

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
    /// Schedules a connection to one or more [`Endpoints`] and then accepts
    /// incoming connections.
    ///
    /// Since ØMQ handles all connections behind the curtain, one cannot know
    /// exactly when the connection is truly established a blocking `send`
    /// or `recv` call is made on that connection.
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// connections that succeeded before the failure.
    ///
    /// See [`zmq_connect`].
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The endpoint's protocol must be supported by the socket..
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if endpoint is invalid)
    /// * [`IncompatTransport`]
    /// * [`CtxTerminated`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_connect`]: http://api.zeromq.org/master:zmq-connect
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`IncompatTransport`]: ../enum.ErrorKind.html#variant.IncompatTransport
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    fn connect<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        let raw_socket = self.raw_socket();
        let mut guard = raw_socket.connected().lock().unwrap();

        for endpoint in endpoints.into_iter() {
            let endpoint: Endpoint = endpoint.into();
            let c_str = CString::new(endpoint.to_zmq()).unwrap();
            connect(raw_socket.as_mut_ptr(), c_str)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            guard.push(endpoint);
            count += 1;
        }
        Ok(())
    }

    /// Returns a snapshot of the list of connected `Endpoint`.
    ///
    /// Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Client, addr::{InprocAddr, Endpoint}};
    /// use std::convert::TryInto;
    ///
    /// let inproc: InprocAddr = "test".try_into()?;
    ///
    /// let client = Client::new()?;
    /// assert!(client.connected().is_empty());
    ///
    /// client.connect(&inproc)?;
    /// let endpoint = Endpoint::from(&inproc);
    /// assert!(client.connected().contains((&endpoint).into()));
    ///
    /// client.disconnect(inproc)?;
    /// assert!(client.connected().is_empty());
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    fn connected(&self) -> Vec<Endpoint> {
        self.raw_socket().connected().lock().unwrap().to_owned()
    }

    /// Disconnect the socket from one or more [`Endpoints`].
    ///
    /// Any outstanding messages physically received from the network but not
    /// yet received by the application are discarded. The behaviour for
    /// discarding messages depends on the value of [`linger`].
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// disconnections that succeeded before the failure.
    ///
    /// See [`zmq_disconnect`].
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The endpoint must be already connected to.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if endpoint is invalid)
    /// * [`NotFound`] (if endpoint not connected to)
    /// * [`CtxTerminated`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_disconnect`]: http://api.zeromq.org/master:zmq-disconnect
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    /// [`linger`]: #method.linger
    fn disconnect<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        let raw_socket = self.raw_socket();
        let mut guard = raw_socket.connected().lock().unwrap();

        for endpoint in endpoints.into_iter() {
            let endpoint = endpoint.into();
            let c_str = CString::new(endpoint.to_zmq()).unwrap();
            disconnect(raw_socket.as_mut_ptr(), c_str)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            let position = guard.iter().position(|e| e == &endpoint).unwrap();
            guard.remove(position);
            count += 1;
        }
        Ok(())
    }

    /// Schedules a bind to one or more [`Endpoints`] and then accepts
    /// incoming connections.
    ///
    /// Since ØMQ handles all connections behind the curtain, one cannot know
    /// exactly when the connection is truly established a blocking `send`
    /// or `recv` call is made on that connection.
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// binds that succeeded before the failure.
    ///
    /// See [`zmq_bind`].
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The transport must be supported by socket type.
    /// * The endpoint must not be in use.
    /// * The endpoint must be local.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if endpoint is invalid)
    /// * [`IncompatTransport`] (if transport is not supported)
    /// * [`AddrInUse`] (if addr already in use)
    /// * [`AddrNotAvailable`] (if not local)
    /// * [`CtxTerminated`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_bind`]: http://api.zeromq.org/master:zmq-bind
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`IncompatTransport`]: ../enum.ErrorKind.html#variant.IncompatTransport
    /// [`AddrInUse`]: ../enum.ErrorKind.html#variant.AddrInUse
    /// [`AddrNotAvailable`]: ../enum.ErrorKind.html#variant.AddrNotAvailable
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    fn bind<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        for endpoint in endpoints.into_iter() {
            let endpoint = endpoint.into();
            let c_str = CString::new(endpoint.to_zmq()).unwrap();
            let raw_socket = self.raw_socket();
            bind(self.raw_socket().as_mut_ptr(), c_str)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            raw_socket.bound().lock().unwrap().push(endpoint);
            count += 1;
        }
        Ok(())
    }
    /// Returns a snapshot of the list of bound `Endpoint`.
    ///
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Radio, addr::{InprocAddr, Endpoint}};
    /// use std::convert::TryInto;
    ///
    /// let first: InprocAddr = "test1".try_into()?;
    /// let second: InprocAddr = "test2".try_into()?;
    ///
    /// let radio = Radio::new()?;
    /// assert!(radio.bound().is_empty());
    ///
    /// radio.bind(vec![&first, &second])?;
    /// {
    ///     let bound = radio.bound();
    ///     let first = Endpoint::from(&first);
    ///     assert!(bound.contains(&first));
    ///     let second = Endpoint::from(&second);
    ///     assert!(bound.contains(&second));
    /// }
    ///
    /// radio.unbind(&first)?;
    /// {
    ///     let bound = radio.bound();
    ///     let first = Endpoint::from(&first);
    ///     assert!(!bound.contains((&first).into()));
    ///     let second = Endpoint::from(&second);
    ///     assert!(bound.contains((&second).into()));
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    fn bound(&self) -> Vec<Endpoint> {
        self.raw_socket().bound().lock().unwrap().to_owned()
    }

    /// Unbinds the socket from one or more [`Endpoints`].
    ///
    /// Any outstanding messages physically received from the network but not
    /// yet received by the application are discarded. The behaviour for
    /// discarding messages depends on the value of [`linger`].
    ///
    /// See [`zmq_unbind`].
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// unbinds that succeeded before the failure.
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The endpoint must be currently bound.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (if usage contract not followed)
    /// * [`CtxTerminated`]
    /// * [`NotFound`] (if endpoint was not bound to)
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_unbind`]: http://api.zeromq.org/master:zmq-unbind
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    /// [`linger`]: #method.linger
    fn unbind<I, E>(&self, endpoints: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        for endpoint in endpoints.into_iter() {
            let endpoint = endpoint.into();
            let c_str = CString::new(endpoint.to_zmq()).unwrap();
            let raw_socket = self.raw_socket();
            unbind(self.raw_socket().as_mut_ptr(), c_str)?;

            let mut bound = raw_socket.bound().lock().unwrap();
            let position = bound.iter().position(|e| e == &endpoint).unwrap();
            bound.remove(position);
        }
        Ok(())
    }

    /// Retrieve the last endpoint connected or bound to.
    ///
    /// This is the only way to retreive the value of a bound `Dynamic` port.
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Server, addr::*};
    /// use std::convert::TryInto;
    ///
    /// // We create a tcp addr with an unspecified port.
    /// // This port will be assigned by the OS upon connection.
    /// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
    /// assert!(addr.host().port().is_unspecified());
    ///
    /// let server = Server::new()?;
    /// assert!(server.last_endpoint()?.is_none());
    ///
    /// server.bind(&addr)?;
    ///
    /// if let Endpoint::Tcp(tcp) = server.last_endpoint()?.unwrap() {
    ///     // The port was indeed assigned by the OS.
    ///     assert!(tcp.host().port().is_specified());
    /// } else {
    ///     unreachable!();
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    fn last_endpoint(&self) -> Result<Option<Endpoint>, Error> {
        let maybe = getsockopt_string(
            self.raw_socket().as_mut_ptr(),
            SocketOption::LastEndpoint,
        )?;

        Ok(maybe.map(|s| Endpoint::from_zmq(s.as_str())))
    }

    /// Retrieve the maximum length of the queue of outstanding peer connections.
    ///
    /// See `ZMQ_BLACKLOG` in [`zmq_getsockopt`].
    ///
    /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
    fn backlog(&self) -> Result<i32, Error> {
        getsockopt_scalar(self.raw_socket().as_mut_ptr(), SocketOption::Backlog)
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
        setsockopt_scalar(
            self.raw_socket().as_mut_ptr(),
            SocketOption::Backlog,
            value,
        )
    }

    /// Retrieves how many milliseconds to wait before timing-out a [`connect`]
    /// call.
    ///
    /// See `ZMQ_CONNECT_TIMEOUT` in [`zmq_getsockopt`].
    ///
    /// [`connect`]: #method.connect
    /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
    fn connect_timeout(&self) -> Result<Option<Duration>, Error> {
        getsockopt_option_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::ConnectTimeout,
            -1,
        )
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
        maybe: Option<Duration>,
    ) -> Result<(), Error> {
        if let Some(ref duration) = maybe {
            assert!(
                duration.as_millis() > 0,
                "number of ms in duration cannot be zero"
            );
        }
        setsockopt_option_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::ConnectTimeout,
            maybe,
            0,
        )
    }

    /// The interval between sending ZMTP heartbeats.
    fn heartbeat_interval(&self) -> Result<Duration, Error> {
        getsockopt_option_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::HeartbeatInterval,
            -1,
        )
        .map(|d| d.unwrap())
    }

    /// Sets the interval between sending ZMTP PINGs (aka. heartbeats).
    ///
    /// # Default Value
    /// `None`
    ///
    /// # Applicable Socket Type
    /// All (connection oriented transports)
    fn set_heartbeat_interval(&self, duration: Duration) -> Result<(), Error> {
        setsockopt_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::HeartbeatInterval,
            duration,
        )
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    fn heartbeat_timeout(&self) -> Result<Duration, Error> {
        getsockopt_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::HeartbeatTimeout,
        )
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    ///
    /// # Default Value
    /// `None`. If `heartbeat_interval` is set, then it uses the same value
    /// by default.
    fn set_heartbeat_timeout(&self, duration: Duration) -> Result<(), Error> {
        setsockopt_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::HeartbeatTimeout,
            duration,
        )
    }

    /// The timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    fn heartbeat_ttl(&self) -> Result<Duration, Error> {
        getsockopt_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::HeartbeatTtl,
        )
    }

    /// Set timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    ///
    /// # Default value
    /// `None`
    fn set_heartbeat_ttl(&self, duration: Duration) -> Result<(), Error> {
        let ms = duration.as_millis();
        if ms > MAX_HB_TTL as u128 {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "duration ms cannot exceed 6553599",
            }));
        }
        setsockopt_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::HeartbeatTtl,
            duration,
        )
    }

    /// Returns the linger period for the socket shutdown.
    fn linger(&self) -> Result<Option<Duration>, Error> {
        getsockopt_option_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::Linger,
            -1,
        )
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
    fn set_linger(&self, maybe: Option<Duration>) -> Result<(), Error> {
        setsockopt_option_duration(
            self.raw_socket().as_mut_ptr(),
            SocketOption::Linger,
            maybe,
            -1,
        )
    }

    fn mechanism(&self) -> Mechanism {
        self.raw_socket().mechanism()
    }

    fn set_mechanism(&self, mechanism: Mechanism) -> Result<(), Error> {
        self.raw_socket().set_mechanism(mechanism)
    }

    fn auth_role(&self) -> AuthRole {
        self.raw_socket().auth_role()
    }

    fn set_auth_role(&self, role: AuthRole) -> Result<(), Error> {
        self.raw_socket().set_auth_role(role)
    }

    fn creds(&self) -> Creds {
        self.raw_socket().creds()
    }

    fn set_creds<C>(&self, creds: C) -> Result<(), Error>
    where
        C: Into<Creds>,
    {
        let creds = creds.into();
        self.raw_socket().set_creds(creds)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SocketConfig {
    pub(crate) connect: Option<Vec<Endpoint>>,
    pub(crate) bind: Option<Vec<Endpoint>>,
    pub(crate) backlog: Option<i32>,
    pub(crate) connect_timeout: Option<Duration>,
    pub(crate) heartbeat_interval: Option<Duration>,
    pub(crate) heartbeat_timeout: Option<Duration>,
    pub(crate) heartbeat_ttl: Option<Duration>,
    pub(crate) linger: Option<Duration>,
    pub(crate) auth_role: Option<AuthRole>,
    pub(crate) mechanism: Option<Mechanism>,
    pub(crate) creds: Option<Creds>,
}

impl SocketConfig {
    pub(crate) fn apply<S: Socket>(
        &self,
        socket: &S,
    ) -> Result<(), failure::Error> {
        if let Some(value) = self.backlog {
            socket.set_backlog(value)?;
        }
        socket.set_connect_timeout(self.connect_timeout)?;
        if let Some(value) = self.heartbeat_interval {
            socket.set_heartbeat_interval(value)?;
        }
        if let Some(value) = self.heartbeat_timeout {
            socket.set_heartbeat_timeout(value)?;
        }
        if let Some(value) = self.heartbeat_ttl {
            socket.set_heartbeat_ttl(value)?;
        }
        socket.set_linger(self.linger)?;
        if let Some(ref creds) = self.creds {
            socket.set_creds(creds)?;
        }
        if let Some(mechanism) = self.mechanism {
            socket.set_mechanism(mechanism)?;
        }
        if let Some(role) = self.auth_role {
            socket.set_auth_role(role)?;
        }

        // We connect as the last step because some socket options
        // only affect subsequent connections.
        if let Some(ref endpoints) = self.connect {
            for endpoint in endpoints {
                socket.connect(endpoint.to_owned())?;
            }
        }
        if let Some(ref endpoints) = self.bind {
            for endpoint in endpoints {
                socket.bind(endpoint.to_owned())?;
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub trait GetSocketConfig: private::Sealed {
    fn socket_config(&self) -> &SocketConfig;

    fn socket_config_mut(&mut self) -> &mut SocketConfig;
}

impl GetSocketConfig for SocketConfig {
    fn socket_config(&self) -> &SocketConfig {
        self
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self
    }
}

/// A set of provided methods for a socket configuration.
pub trait ConfigureSocket: GetSocketConfig {
    fn connect(&self) -> Option<&[Endpoint]> {
        self.socket_config().connect.as_ref().map(Vec::as_slice)
    }

    fn set_connect<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let maybe: Option<Vec<Endpoint>> =
            maybe.map(|e| e.into_iter().map(Into::into).collect());
        self.socket_config_mut().connect = maybe;
    }

    fn bind(&self) -> Option<&[Endpoint]> {
        self.socket_config().bind.as_ref().map(Vec::as_slice)
    }

    fn set_bind<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let maybe: Option<Vec<Endpoint>> =
            maybe.map(|e| e.into_iter().map(Into::into).collect());
        self.socket_config_mut().bind = maybe;
    }

    fn backlog(&self) -> Option<i32> {
        self.socket_config().backlog
    }

    fn set_backlog(&mut self, maybe: Option<i32>) {
        self.socket_config_mut().backlog = maybe;
    }

    fn connect_timeout(&self) -> Option<Duration> {
        self.socket_config().connect_timeout
    }

    fn set_connect_timeout(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().connect_timeout = maybe;
    }

    fn heartbeat_interval(&self) -> Option<Duration> {
        self.socket_config().heartbeat_interval
    }

    fn set_heartbeat_interval(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().heartbeat_interval = maybe;
    }

    fn heartbeat_timeout(&self) -> Option<Duration> {
        self.socket_config().heartbeat_timeout
    }

    fn set_heartbeat_timeout(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().heartbeat_timeout = maybe;
    }

    fn heartbeat_ttl(&self) -> Option<Duration> {
        self.socket_config().heartbeat_ttl
    }

    fn set_heartbeat_ttl(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().heartbeat_ttl = maybe;
    }

    fn linger(&self) -> Option<Duration> {
        self.socket_config().linger
    }

    fn set_linger(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().linger = maybe;
    }

    fn auth_role(&self) -> Option<AuthRole> {
        self.socket_config().auth_role
    }

    fn set_auth_role(&mut self, maybe: Option<AuthRole>) {
        self.socket_config_mut().auth_role = maybe;
    }

    fn mechanism(&self) -> Option<Mechanism> {
        self.socket_config().mechanism
    }

    fn set_mechanism(&mut self, maybe: Option<Mechanism>) {
        self.socket_config_mut().mechanism = maybe;
    }

    fn creds(&self) -> Option<&Creds> {
        self.socket_config().creds.as_ref()
    }

    fn set_creds(&mut self, maybe: Option<Creds>) {
        self.socket_config_mut().creds = maybe;
    }
}

impl ConfigureSocket for SocketConfig {}

/// A set of provided methods for a socket builder.
pub trait BuildSocket: GetSocketConfig + Sized {
    fn connect<I, E>(&mut self, endpoints: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        self.socket_config_mut().set_connect(Some(endpoints));
        self
    }

    fn bind<I, E>(&mut self, endpoints: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        self.socket_config_mut().set_bind(Some(endpoints));
        self
    }

    fn backlog(&mut self, len: i32) -> &mut Self {
        self.socket_config_mut().set_backlog(Some(len));
        self
    }

    fn connect_timeout(&mut self, maybe: Option<Duration>) -> &mut Self {
        self.socket_config_mut().set_connect_timeout(maybe);
        self
    }

    fn heartbeat_interval(&mut self, duration: Duration) -> &mut Self {
        self.socket_config_mut()
            .set_heartbeat_interval(Some(duration));
        self
    }

    fn heartbeat_timeout(&mut self, duration: Duration) -> &mut Self {
        self.socket_config_mut()
            .set_heartbeat_timeout(Some(duration));
        self
    }

    fn heartbeat_ttl(&mut self, duration: Duration) -> &mut Self {
        self.socket_config_mut().set_heartbeat_ttl(Some(duration));
        self
    }

    fn linger(&mut self, maybe: Option<Duration>) -> &mut Self {
        self.socket_config_mut().set_linger(maybe);
        self
    }

    fn auth_role(&mut self, role: AuthRole) -> &mut Self {
        self.socket_config_mut().set_auth_role(Some(role));
        self
    }

    fn mechanism(&mut self, mechanism: Mechanism) -> &mut Self {
        self.socket_config_mut().set_mechanism(Some(mechanism));
        self
    }

    fn creds(&mut self, creds: Creds) -> &mut Self {
        self.socket_config_mut().set_creds(Some(creds));
        self
    }
}
