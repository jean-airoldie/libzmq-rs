use crate::{
    ctx::Ctx,
    endpoint::Endpoint,
    error::{msg_from_errno, Error, ErrorKind},
    msg::Msg,
    sockopt::*,
};

use libzmq_sys as sys;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{
    ffi::CString,
    os::{
        raw::{c_int, c_void},
        unix::io::RawFd,
    },
    sync::Arc,
    time::Duration,
};

const MAX_HB_TTL: i64 = 6_553_599;

/// Prevent users from implementing the AsRawSocket trait.
mod private {
    pub trait Sealed {}
    impl Sealed for super::Client {}
    impl Sealed for super::ClientConfig {}
    impl Sealed for super::Server {}
    impl Sealed for super::ServerConfig {}
    impl Sealed for super::Radio {}
    impl Sealed for super::RadioConfig {}
    impl Sealed for super::Dish {}
    impl Sealed for super::DishConfig {}
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SocketConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    backlog: Option<i32>,
    connect_timeout: Option<Duration>,
    heartbeat_interval: Option<Duration>,
    heartbeat_timeout: Option<Duration>,
    heartbeat_ttl: Option<Duration>,
}

#[doc(hidden)]
pub trait AsSocketConfig: private::Sealed {
    fn as_socket_config(&self) -> &SocketConfig;

    fn as_mut_as_socket_config(&mut self) -> &mut SocketConfig;
}

/// The set of shared socket configuration methods.
pub trait SocketBuilder: AsSocketConfig {
    fn connect(&mut self, endpoints: Vec<Endpoint>) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.connect = Some(endpoints);
        self
    }

    fn bind(&mut self, endpoints: Vec<Endpoint>) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.bind = Some(endpoints);
        self
    }

    fn backlog(&mut self, len: i32) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.backlog = Some(len);
        self
    }

    fn connect_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.connect_timeout = maybe_duration;
        self
    }

    fn heartbeat_interval(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.heartbeat_interval = maybe_duration;
        self
    }

    fn heartbeat_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.heartbeat_timeout = maybe_duration;
        self
    }

    fn heartbeat_ttl(&mut self, maybe_duration: Option<Duration>) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.heartbeat_ttl = maybe_duration;
        self
    }

    fn apply<S: Socket>(&self, socket: &S) -> Result<(), Error<()>> {
        let config = self.as_socket_config();

        if let Some(ref endpoints) = config.connect {
            for endpoint in endpoints {
                socket.connect(endpoint)?;
            }
        }
        if let Some(ref endpoints) = config.bind {
            for endpoint in endpoints {
                socket.bind(endpoint)?;
            }
        }
        if let Some(value) = config.backlog {
            socket.set_backlog(value)?;
        }
        socket.set_connect_timeout(config.connect_timeout)?;

        Ok(())
    }
}

macro_rules! impl_config_trait {
    ($name:ident) => {
        impl AsSocketConfig for $name {
            #[doc(hidden)]
            fn as_socket_config(&self) -> &SocketConfig {
                &self.inner
            }

            #[doc(hidden)]
            fn as_mut_as_socket_config(&mut self) -> &mut SocketConfig {
                &mut self.inner
            }
        }

        impl SocketBuilder for $name {}
    };
}

fn connect(
    mut_raw_socket: *mut c_void,
    c_str: CString,
) -> Result<(), Error<()>> {
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

fn bind(mut_raw_socket: *mut c_void, c_str: CString) -> Result<(), Error<()>> {
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
) -> Result<(), Error<()>> {
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

fn unbind(
    mut_raw_socket: *mut c_void,
    c_str: CString,
) -> Result<(), Error<()>> {
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

#[doc(hidden)]
pub trait AsRawSocket: private::Sealed {
    fn as_raw_socket(&self) -> *const c_void;

    fn as_mut_raw_socket(&self) -> *mut c_void;
}

/// Methods shared by all thread-safe sockets.
pub trait Socket: AsRawSocket {
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
    fn connect<E>(&self, endpoint: E) -> Result<(), Error<()>>
    where
        E: AsRef<Endpoint>,
    {
        let c_str = CString::new(endpoint.as_ref().to_string()).unwrap();
        connect(self.as_mut_raw_socket(), c_str)
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
    fn disconnect<E>(&self, endpoint: &E) -> Result<(), Error<()>>
    where
        E: AsRef<Endpoint>,
    {
        let c_str = CString::new(endpoint.as_ref().to_string()).unwrap();
        disconnect(self.as_mut_raw_socket(), c_str)
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
    fn bind<E>(&self, endpoint: &E) -> Result<(), Error<()>>
    where
        E: AsRef<Endpoint>,
    {
        let c_str = CString::new(endpoint.as_ref().to_string()).unwrap();
        bind(self.as_mut_raw_socket(), c_str)
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
    fn unbind<E>(&self, endpoint: &E) -> Result<(), Error<()>>
    where
        E: AsRef<Endpoint>,
    {
        let c_str = CString::new(endpoint.as_ref().to_string()).unwrap();
        unbind(self.as_mut_raw_socket(), c_str)
    }

    /// Retrieve the maximum length of the queue of outstanding peer connections.
    ///
    /// See `ZMQ_BLACKLOG` in [`zmq_getsockopt`].
    ///
    /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
    fn backlog(&self) -> Result<i32, Error<()>> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.as_raw_socket() as *mut _;
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
    fn set_backlog(&self, value: i32) -> Result<(), Error<()>> {
        setsockopt_scalar(
            self.as_mut_raw_socket(),
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
    fn connect_timeout(&self) -> Result<Option<Duration>, Error<()>> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        let maybe_duration =
            getsockopt_duration(mut_raw_socket, SocketOption::ConnectTimeout)?;
        if let Some(duration) = maybe_duration {
            if duration.as_millis() > 0 {
                return Ok(Some(duration));
            }
        }
        Ok(None)
    }

    /// Sets how many milliseconds to wait before timing-out a [`connect`] call
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
    ) -> Result<(), Error<()>> {
        if let Some(ref duration) = maybe_duration {
            assert!(
                duration.as_millis() > 0,
                "number of ms in duration cannot be zero"
            );
        }
        // This is safe the call does not actually mutate the socket.
        setsockopt_duration(
            self.as_mut_raw_socket(),
            SocketOption::ConnectTimeout,
            maybe_duration,
        )
    }

    /// Retrieve the file descriptor associated with the specified socket.
    ///
    /// The returned file descriptor is intended for use with a poll or similar
    /// system call only. Applications must never attempt to read or write data
    /// to it directly, neither should they try to close it.
    ///
    /// See `ZMQ_FD` in [`zmq_getsockopt`].
    ///
    /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
    fn fd(&self) -> Result<RawFd, Error<()>> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        getsockopt_scalar(mut_raw_socket, SocketOption::FileDescriptor)
    }

    /// The interval between sending ZMTP heartbeats.
    fn heartbeat_interval(&self) -> Result<Option<Duration>, Error<()>> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::HeartbeatInterval)
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
    ) -> Result<(), Error<()>> {
        setsockopt_duration(
            self.as_mut_raw_socket(),
            SocketOption::HeartbeatInterval,
            maybe_duration,
        )
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    fn heartbeat_timeout(&self) -> Result<Option<Duration>, Error<()>> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::HeartbeatTimeout)
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
    ) -> Result<(), Error<()>> {
        setsockopt_duration(
            self.as_mut_raw_socket(),
            SocketOption::HeartbeatTimeout,
            maybe_duration,
        )
    }

    /// The timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    fn heartbeat_ttl(&self) -> Result<Option<Duration>, Error<()>> {
        // This is safe the call does not actually mutate the socket.
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::HeartbeatTtl)
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
    ) -> Result<(), Error<()>> {
        if let Some(ref duration) = maybe_duration {
            let ms = duration.as_millis();
            if ms <= MAX_HB_TTL as u128 {
                return Err(Error::new(ErrorKind::InvalidInput {
                    msg: "duration ms cannot exceed 6553599",
                }));
            }
        }
        setsockopt_duration(
            self.as_mut_raw_socket(),
            SocketOption::HeartbeatTtl,
            maybe_duration,
        )
    }
}

fn send(
    mut_raw_socket: *mut c_void,
    mut msg: Msg,
    no_block: bool,
) -> Result<(), Error<Msg>> {
    let mut_msg_ptr = msg.as_mut_ptr();
    let rc = unsafe {
        sys::zmq_msg_send(mut_msg_ptr, mut_raw_socket, no_block as c_int)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EAGAIN => {
                    Error::with_content(ErrorKind::WouldBlock, msg)
                }
                errno::ENOTSUP => {
                    panic!("send is not supported by socket type")
                }
                errno::EINVAL => panic!(
                    "multipart messages are not supported by socket type"
                ),
                errno::EFSM => panic!(
                    "operation cannot be completed in current socket state"
                ),
                errno::ETERM => {
                    Error::with_content(ErrorKind::CtxTerminated, msg)
                }
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => {
                    Error::with_content(ErrorKind::Interrupted, msg)
                }
                errno::EFAULT => panic!("invalid message"),
                errno::EHOSTUNREACH => {
                    Error::with_content(ErrorKind::HostUnreachable, msg)
                }
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

/// Send messages in a thread-safe fashion.
///
/// Does not support multipart messages.
pub trait SendMsg: AsRawSocket {
    /// Push a message into the outgoing socket queue.
    ///
    /// This operation might block if the socket is in mute state.
    ///
    /// If the message is a `Msg`, `Vec<u8>`, `[u8]`, or a `String`, it is not copied.
    ///
    /// # Success
    /// The message was queued and now belongs to ØMQ
    ///
    /// # Error
    /// In case of an error, the message is not queued and
    /// the ownership is returned.
    ///
    /// ## Possible Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`HostUnreachable`] (only for [`Server`] socket)
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`HostUnreachable`]: ../enum.ErrorKind.html#variant.HostUnreachable
    /// [`Server`]: struct.Server.html
    fn send<M>(&self, sendable: M) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        let msg: Msg = sendable.into();
        send(self.as_mut_raw_socket(), msg, false)
    }

    /// Push a message into the outgoing socket queue without blocking.
    ///
    /// This polls the socket so see if the socket is in mute state, if it
    /// is it errors with [`WouldBlock`], otherwise is pushes the message into
    /// the outgoing queue.
    ///
    /// If the message is a `Msg`, `Vec<u8>`, `[u8]`, or a `String`, it is not copied.
    ///
    /// # Success
    /// The message was queued and now belongs to ØMQ
    ///
    /// # Error
    /// In case of an error, the message is not queued and
    /// the ownership is returned.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`HostUnreachable`] (only for [`Server`] socket)
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`HostUnreachable`]: ../enum.ErrorKind.html#variant.HostUnreachable
    /// [`Server`]: struct.Server.html
    fn send_poll<M>(&self, sendable: M) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        let msg: Msg = sendable.into();
        send(self.as_mut_raw_socket(), msg, true)
    }

    /// The high water mark for outbound messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// outstanding messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    fn send_high_water_mark(&self) -> Result<Option<i32>, Error<()>> {
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        let limit =
            getsockopt_scalar(mut_raw_socket, SocketOption::SendHighWaterMark)?;

        if limit == 0 {
            Ok(None)
        } else {
            Ok(Some(limit))
        }
    }

    /// Set the high water mark for outbound messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// outstanding messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    ///
    /// A value of `None` means no limit.
    ///
    /// # Default value
    /// 1000
    fn set_send_high_water_mark(
        &self,
        maybe_limit: Option<i32>,
    ) -> Result<(), Error<()>> {
        match maybe_limit {
            Some(limit) => {
                assert!(limit != 0, "high water mark cannot be zero");
                setsockopt_scalar(
                    self.as_mut_raw_socket(),
                    SocketOption::SendHighWaterMark,
                    limit,
                )
            }
            None => setsockopt_scalar(
                self.as_mut_raw_socket(),
                SocketOption::SendHighWaterMark,
                0,
            ),
        }
    }

    /// Sets the timeout for `send` operation on the socket.
    ///
    /// If the value is 0, `send` will return immediately, with a EAGAIN
    /// error if the message cannot be sent. If the value is `None`, it
    /// will block until the message is sent. For all other values, it will
    /// try to send the message for that amount of time before returning
    /// with an EAGAIN error.
    fn send_timeout(&self) -> Result<Option<Duration>, Error<()>> {
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::SendTimeout)
    }

    /// Sets the timeout for `send` operation on the socket.
    ///
    /// If the value is 0, `send` will return immediately, with a EAGAIN
    /// error if the message cannot be sent. If the value is `None`, it
    /// will block until the message is sent. For all other values, it will
    /// try to send the message for that amount of time before returning
    /// with an EAGAIN error.
    ///
    /// # Default Value
    /// `None`
    fn set_send_timeout(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error<()>> {
        setsockopt_duration(
            self.as_mut_raw_socket(),
            SocketOption::SendTimeout,
            maybe_duration,
        )
    }
}

fn recv(
    mut_raw_socket: *mut c_void,
    msg: &mut Msg,
    no_block: bool,
) -> Result<(), Error<()>> {
    let rc = unsafe {
        sys::zmq_msg_recv(msg.as_mut_ptr(), mut_raw_socket, no_block as c_int)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
                errno::ENOTSUP => panic!("recv not supported by socket type"),
                errno::EFSM => panic!(
                    "operation cannot be completed in current socket state"
                ),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::EFAULT => panic!("invalid message"),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

/// Receive atomic messages in an immutable, thread-safe fashion.
///
/// Does not support multipart messages.
pub trait RecvMsg: AsRawSocket {
    /// Retreive a message from the inbound socket queue.
    ///
    /// This operation might block until the socket receives a message.
    ///
    /// # Error
    /// No message from the inbound queue is lost if there is an error.
    ///
    /// ## Possible Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn recv(&self, msg: &mut Msg) -> Result<(), Error<()>> {
        recv(self.as_mut_raw_socket(), msg, false)
    }

    /// Retreive a message from the inbound socket queue without blocking.
    ///
    /// This polls the socket to determine there is at least on inbound message in
    /// the socket queue. If there is, it retuns it, otherwise it errors with
    /// [`WouldBlock`].
    ///
    /// # Error
    /// No message from the inbound queue is lost if there is an error.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn recv_poll(&self, msg: &mut Msg) -> Result<(), Error<()>> {
        recv(self.as_mut_raw_socket(), msg, true)
    }

    /// A convenience function that allocates a [`Msg`] with the same properties
    /// as [`recv`].
    ///
    /// [`recv`]: #method.recv
    /// [`Msg`]: ../msg/struct.Msg.html
    fn recv_msg(&self) -> Result<Msg, Error<()>> {
        let mut msg = Msg::new();
        self.recv(&mut msg)?;

        Ok(msg)
    }

    /// A convenience function that allocates a [`Msg`] with the same properties
    /// as [`recv_poll`].
    ///
    /// [`recv_poll`]: #method.recv
    /// [`Msg`]: ../msg/struct.Msg.html
    fn recv_msg_poll(&self) -> Result<Msg, Error<()>> {
        let mut msg = Msg::new();
        self.recv_poll(&mut msg)?;

        Ok(msg)
    }

    /// The high water mark for incoming messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// incoming messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    fn recv_high_water_mark(&self) -> Result<Option<i32>, Error<()>> {
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        let limit =
            getsockopt_scalar(mut_raw_socket, SocketOption::RecvHighWaterMark)?;

        if limit == 0 {
            Ok(None)
        } else {
            Ok(Some(limit))
        }
    }

    /// Set the high water mark for inbound messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// outstanding messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    ///
    /// A value of `None` means no limit.
    ///
    /// # Default value
    /// 1000
    fn set_recv_high_water_mark(
        &self,
        maybe_limit: Option<i32>,
    ) -> Result<(), Error<()>> {
        match maybe_limit {
            Some(limit) => {
                assert!(limit != 0, "high water mark cannot be zero");
                setsockopt_scalar(
                    self.as_mut_raw_socket(),
                    SocketOption::RecvHighWaterMark,
                    limit,
                )
            }
            None => setsockopt_scalar(
                self.as_mut_raw_socket(),
                SocketOption::RecvHighWaterMark,
                0,
            ),
        }
    }

    /// Sets the timeout for `recv` operation on the socket.
    ///
    /// If the value is 0, `recv` will return immediately, with a EAGAIN
    /// error if the message cannot be sent. If the value is `None`, it
    /// will block until the message is sent. For all other values, it will
    /// try to recv the message for that amount of time before returning
    /// with an EAGAIN error.
    fn recv_timeout(&self) -> Result<Option<Duration>, Error<()>> {
        let mut_raw_socket = self.as_raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::RecvTimeout)
    }

    /// Sets the timeout for `recv` operation on the socket.
    ///
    /// If the value is 0, `recv` will return immediately, with a EAGAIN
    /// error if the message cannot be sent. If the value is `None`, it
    /// will block until the message is sent. For all other values, it will
    /// try to `recv` the message for that amount of time before returning
    /// with an EAGAIN error.
    ///
    /// # Default Value
    /// `None`
    fn set_recv_timeout(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error<()>> {
        setsockopt_duration(
            self.as_mut_raw_socket(),
            SocketOption::RecvTimeout,
            maybe_duration,
        )
    }
}

enum RawSocketType {
    Client,
    Server,
    Radio,
    Dish,
}

impl Into<c_int> for RawSocketType {
    fn into(self) -> c_int {
        match self {
            RawSocketType::Client => sys::ZMQ_CLIENT as c_int,
            RawSocketType::Server => sys::ZMQ_SERVER as c_int,
            RawSocketType::Radio => sys::ZMQ_RADIO as c_int,
            RawSocketType::Dish => sys::ZMQ_DISH as c_int,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct RawSocket {
    ctx: Ctx,
    socket: *mut c_void,
}

impl RawSocket {
    fn new(sock_type: RawSocketType) -> Result<Self, Error<()>> {
        let ctx = Ctx::global().clone();

        Self::with_ctx(sock_type, ctx)
    }

    fn with_ctx(sock_type: RawSocketType, ctx: Ctx) -> Result<Self, Error<()>> {
        let socket = unsafe { sys::zmq_socket(ctx.as_ptr(), sock_type.into()) };

        if socket.is_null() {
            let errno = unsafe { sys::zmq_errno() };
            let err = match errno {
                errno::EINVAL => panic!("invalid socket type"),
                errno::EFAULT => panic!("invalid ctx"),
                errno::EMFILE => Error::new(ErrorKind::SocketLimit),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            Ok(Self { ctx, socket })
        }
    }
}

impl Drop for RawSocket {
    /// Close the ØMQ socket.
    ///
    /// See [`zmq_close`].
    ///
    /// [`zmq_close`]: http://api.zeromq.org/master:zmq-close
    fn drop(&mut self) {
        let rc = unsafe { sys::zmq_close(self.socket) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            match errno {
                errno::EFAULT => panic!("socket invalid"),
                _ => panic!(msg_from_errno(errno)),
            }
        }
    }
}

/// Implement the shared methods for a socket.
macro_rules! impl_socket_methods {
    ($name:ident, $sname:expr) => {
            /// Create a `
            #[doc = $sname]
            /// ` socket from the [`global context`]
            ///
            /// # Returned Error Variants
            /// * [`CtxTerminated`]
            /// * [`SocketLimit`]
            ///
            /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
            /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
            /// [`global context`]: ../ctx/struct.Ctx.html#method.global
            pub fn new() -> Result<Self, Error<()>> {
                let inner = Arc::new(RawSocket::new(RawSocketType::$name)?);

                Ok(Self {
                    inner,
                })
            }

            /// Create a `
            #[doc = $sname]
            /// ` socket from a specific context.
            ///
            /// # Returned Error Variants
            /// * [`CtxTerminated`]
            /// * [`SocketLimit`]
            ///
            /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
            /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
            pub fn with_ctx(ctx: Ctx) -> Result<Self, Error<()>> {
                let inner = Arc::new(
                    RawSocket::with_ctx(RawSocketType::$name, ctx)?
                );

                Ok(Self {
                    inner,
                })
            }

            /// Returns a reference to the context of the socket.
            pub fn ctx(&self) -> &Ctx {
                &self.inner.ctx
            }

    };

    ($name:tt) => {
        impl_socket_methods!($name, stringify!($name));
    };
}

/// Implement the Socket trait.
macro_rules! impl_socket_trait {
    ($name:ident) => {
        impl AsRawSocket for $name {
            fn as_raw_socket(&self) -> *const c_void {
                self.inner.socket
            }

            // This is safe since this socket is thread safe.
            fn as_mut_raw_socket(&self) -> *mut c_void {
                self.inner.socket as *mut _
            }
        }

        impl Socket for $name {}
    };
}

/// A `Client` socket is used for advanced request-reply messaging.
///
/// `Client` sockets are threadsafe and can be used from multiple threads at the
/// same time. Note that replies from a `Server` socket will go to the first
/// client thread that calls `recv`. If you need to get replies back to the
/// originating thread, use one `Client` socket per thread.
///
/// When a `Client` socket is connected to multiple sockets, outgoing
/// messages are distributed between connected peers on a round-robin basis.
/// Likewise, the `Client` socket receives messages fairly from each connected peer.
///
/// `Client` sockets do not accept the `MORE` flag on sends. This limits them to
/// single part data.
///
/// # Mute State
/// When `Client` socket enters the mute state due to having reached the high water
/// mark, or if there are no peers at all, then any `send operations
/// on the socket shall block unitl the mute state ends or at least one peer becomes
/// available for sending; messages are not discarded.
///
/// # Usage Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::prelude::*;
///
/// let endpoint: Endpoint = "inproc://test".parse()?;
///
/// // Lets illustrate a request reply pattern using 2 client messaging
/// // each other.
/// let mut first = Client::new()?;
/// let mut second = Client::new()?;
///
/// first.bind(&endpoint)?;
/// second.connect(&endpoint)?;
///
/// // Lets do the whole request-reply thing.
/// first.send("request")?;
///
/// let mut msg = second.recv_msg()?;
/// assert_eq!("request", msg.to_str()?);
///
/// second.send("reply")?;
///
/// first.recv(&mut msg)?;
/// assert_eq!("reply", msg.to_str()?);
///
/// // We can send as many replies as we want. We don't need to follow
/// // a strict one request equals one reply pattern.
/// second.send("another reply")?;
///
/// first.recv(&mut msg)?;
/// assert_eq!("another reply", msg.to_str()?);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Server`], [`Client`] |
/// | Direction                 | Bidirectional          |
/// | Send/receive pattern      | Unrestricted           |
/// | Outgoing routing strategy | Round-robin            |
/// | Incoming routing strategy | Fair-queued            |
/// | Action in mute state      | Block                  |
///
/// [`Server`]: struct.Server.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    inner: Arc<RawSocket>,
}

impl Client {
    impl_socket_methods!(Client);
}

impl_socket_trait!(Client);

impl SendMsg for Client {}
impl RecvMsg for Client {}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

/// A builder for a `Client`.
///
/// Especially helpfull in config files.
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientConfig {
    inner: SocketConfig,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Client, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Client, Error<()>> {
        let client = Client::with_ctx(ctx)?;
        self.apply(&client)?;

        Ok(client)
    }
}

impl_config_trait!(ClientConfig);

/// A `Server` socket is a socket used for advanced request-reply messaging.
///
/// `Server` sockets are threadsafe and do not accept the [`MORE`] flag.
///
/// A `Server` socket talks to a set of [`Client`] sockets. The [`Client`] must
/// first initiate the conversation, which generates a [`routing_id`] associated
/// with the connection. Each message received from a `Server` will have this
/// [`routing_id`]. To send messages back to the client, you must
/// [`set_routing_id`] on the messages. If the [`routing_id`] is not specified, or
/// does not refer to a connected client peer, the send call will fail with
/// [`HostUnreachable`].
///
/// # Mute State
/// When a `Server` socket enters the mute state due to having reached the high
/// water mark for all clients, or if there are no clients at
/// all, then any `send` operations on the socket shall block until the mute
/// state ends or at least one downstream node becomes available for sending;
/// messages are not discarded.
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Client`]             |
/// | Direction                 | Bidirectional          |
/// | Pattern                   | Unrestricted           |
/// | Incoming routing strategy | Fair-queued            |
/// | Outgoing routing strategy | See text               |
/// | Action in mute state      | Block                  |
///
/// # Usage Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::prelude::*;
///
/// let endpoint: Endpoint = "inproc://test".parse()?;
///
/// let client = Client::new()?;
/// let server = Server::new()?;
///
/// client.connect(&endpoint)?;
/// server.bind(&endpoint)?;
///
/// // The client initiates the conversation so it is assigned a `routing_id`.
/// client.send("request")?;
/// let msg = server.recv_msg()?;
/// assert_eq!("request", msg.to_str()?);
/// let routing_id = msg.routing_id().expect("no routing id");
///
/// // Using this `routing_id`, we can now route as many replies as we
/// // want to the client.
/// let mut msg: Msg = "reply 1".into();
/// msg.set_routing_id(routing_id);
/// server.send(msg)?;
/// let mut msg: Msg = "reply 2".into();
/// msg.set_routing_id(routing_id);
/// server.send(msg)?;
///
/// // The `routing_id` is discarted when the message is sent to the client.
/// let mut msg = client.recv_msg()?;
/// assert_eq!("reply 1", msg.to_str()?);
/// assert!(msg.routing_id().is_none());
/// client.recv(&mut msg)?;
/// assert_eq!("reply 2", msg.to_str()?);
/// assert!(msg.routing_id().is_none());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`MORE`]: constant.MORE.html
/// [`Client`]: struct.Client.html
/// [`routing_id`]: ../msg/struct.Msg.html#method.routing_id
/// [`set_routing_id`]: ../msg/struct.Msg.html#method.set_routing_id
/// [`HostUnreachable`]: ../enum.ErrorKind.html#variant.host-unreachable
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    inner: Arc<RawSocket>,
}

impl Server {
    impl_socket_methods!(Server);
}

impl_socket_trait!(Server);

impl SendMsg for Server {}
impl RecvMsg for Server {}

unsafe impl Send for Server {}
unsafe impl Sync for Server {}

/// A builder for a `Server`.
///
/// Especially helpfull in config files.
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerConfig {
    inner: SocketConfig,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Server, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Server, Error<()>> {
        let server = Server::with_ctx(ctx)?;
        self.apply(&server)?;

        Ok(server)
    }
}

impl_config_trait!(ServerConfig);

/// A `Radio` socket is used by a publisher to distribute data to [`Dish`]
/// sockets.
///
/// Each message belong to a group specified with [`set_group`].
/// Messages are distributed to all members of a group.
///
/// # Mute State
/// When a `Radio` socket enters the mute state due to having reached the
/// high water mark for a subscriber, then any messages that would be sent to
/// the subscriber in question shall instead be dropped until the mute state ends.
///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::prelude::*;
///
/// let endpoint: Endpoint = "inproc://test".parse().unwrap();
///
/// // We create our sockets.
/// let radio = Radio::new()?;
/// // We configure the radio so that it doesnt drop in mute state.
/// // However this means that a slow `Dish` would slow down
/// // the `Radio`. We use this is this example because `connect`
/// // takes a few milliseconds, enough for the `Radio` to drop a few messages.
/// radio.set_no_drop(true)?;
/// let first = Dish::new()?;
/// let second = Dish::new()?;
///
/// // We connect them.
/// radio.bind(&endpoint)?;
/// first.connect(&endpoint)?;
/// second.connect(&endpoint)?;
///
/// // Each dish will only receive messages from that group.
/// first.join("first group")?;
/// second.join("second group")?;
///
/// // Lets publish some messages to subscribers.
/// let mut msg: Msg = "first msg".into();
/// msg.set_group("first group")?;
/// radio.send(msg)?;
/// let mut msg: Msg = "second msg".into();
/// msg.set_group("second group")?;
/// radio.send(msg)?;
///
/// // Lets receive the publisher's messages.
/// let mut msg = first.recv_msg()?;
/// assert_eq!("first msg", msg.to_str().unwrap());
/// let err = first.recv_poll(&mut msg).unwrap_err();
/// // Only the message from the first group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// second.recv(&mut msg)?;
/// assert_eq!("second msg", msg.to_str().unwrap());
/// let err = first.recv_poll(&mut msg).unwrap_err();
/// // Only the message from the second group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value          |
/// |:-------------------------:|:--------------:|
/// | Compatible peer sockets   | [`Dish`]       |
/// | Direction                 | Unidirectional |
/// | Send/receive pattern      | Send only      |
/// | Incoming routing strategy | N/A            |
/// | Outgoing routing strategy | Fan out        |
/// | Action in mute state      | Drop           |
///
/// [`Dish`]: struct.Dish.html
/// [`set_group`]: ../struct.Msg.html#method.set_group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Radio {
    inner: Arc<RawSocket>,
}

impl Radio {
    impl_socket_methods!(Radio);

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> Result<bool, Error<()>> {
        getsockopt_bool(self.as_mut_raw_socket(), SocketOption::NoDrop)
    }

    /// Sets the socket's behaviour to block instead of drop messages when
    /// in the `mute state`.
    ///
    /// # Default value
    /// `false`
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`send_high_water_mark`]: #method.send_high_water_mark
    pub fn set_no_drop(&self, enabled: bool) -> Result<(), Error<()>> {
        setsockopt_bool(self.as_mut_raw_socket(), SocketOption::NoDrop, enabled)
    }
}

impl_socket_trait!(Radio);

impl SendMsg for Radio {}

unsafe impl Send for Radio {}
unsafe impl Sync for Radio {}

/// A builder for a `Radio`.
///
/// Especially helpfull in config files.
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RadioConfig {
    inner: SocketConfig,
    no_drop: Option<bool>,
}

impl RadioConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Radio, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Radio, Error<()>> {
        let radio = Radio::with_ctx(ctx)?;
        self.apply(&radio)?;

        if let Some(enabled) = self.no_drop {
            radio.set_no_drop(enabled)?;
        }

        Ok(radio)
    }
}

impl_config_trait!(RadioConfig);

/// A `Dish` socket is used by a subscriber to subscribe to groups distributed
/// by a [`Radio`].
///
/// Initially a ZMQ_DISH socket is not subscribed to any groups, use [`join`]
/// to join a group.
///
/// # Summary of Characteristics
/// | Characteristic            | Value          |
/// |:-------------------------:|:--------------:|
/// | Compatible peer sockets   | [`Radio`]      |
/// | Direction                 | Unidirectional |
/// | Send/receive pattern      | Receive only   |
/// | Incoming routing strategy | Fair-queued    |
/// | Outgoing routing strategy | N/A            |
///
/// [`Radio`]: struct.Radio.html
/// [`join`]: #method.join
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    inner: Arc<RawSocket>,
}

impl Dish {
    impl_socket_methods!(Dish);

    /// Joins the specified group.
    ///
    /// # Usage Contract
    /// * The group `str` must be at most 15 characters.
    /// * Each group can be subscribed at most once.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`InvalidInput`] (if contract is not followed)
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    pub fn join<S>(&self, group: S) -> Result<(), Error<()>>
    where
        S: AsRef<str>,
    {
        let c_str = CString::new(group.as_ref()).unwrap();
        let rc =
            unsafe { sys::zmq_join(self.as_mut_raw_socket(), c_str.as_ptr()) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = {
                match errno {
                    errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                        msg: "invalid group",
                    }),
                    errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                    errno::EINTR => Error::new(ErrorKind::Interrupted),
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

    /// Leave the specified group.
    ///
    /// # Usage Contract
    /// * The group `str` must be at most 15 characters.
    /// * The group must be already joined.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`InvalidInput`] (if contract is not followed)
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    pub fn leave<S>(&self, group: S) -> Result<(), Error<()>>
    where
        S: AsRef<str>,
    {
        let c_str = CString::new(group.as_ref()).unwrap();
        let rc =
            unsafe { sys::zmq_leave(self.as_mut_raw_socket(), c_str.as_ptr()) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = {
                match errno {
                    errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                        msg: "invalid group",
                    }),
                    errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                    errno::EINTR => Error::new(ErrorKind::Interrupted),
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
}

impl_socket_trait!(Dish);

impl RecvMsg for Dish {}

unsafe impl Send for Dish {}
unsafe impl Sync for Dish {}

/// A builder for a `Dish`.
///
/// Especially helpfull in config files.
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DishConfig {
    inner: SocketConfig,
    groups: Option<Vec<String>>,
}

impl DishConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Dish, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Dish, Error<()>> {
        let dish = Dish::with_ctx(ctx)?;
        self.apply(&dish)?;

        if let Some(ref groups) = self.groups {
            for group in groups {
                dish.join(group)?;
            }
        }
        Ok(dish)
    }
}

impl_config_trait!(DishConfig);

/// The possible socket types.
///
/// # Note
/// This error type is non-exhaustive and could have additional variants
/// added in future. Therefore, when matching against variants of
/// non-exhaustive enums, an extra wildcard arm must be added to account
/// for any future variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketType {
    Radio(Radio),
    Dish(Dish),
    Server(Server),
    Client(Client),
}
