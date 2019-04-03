use crate::{
    ctx::Ctx,
    endpoint::Endpoint,
    error::{msg_from_errno, Error, ErrorKind},
    msg::Msg,
    sockopt::*,
};

use libzmq_sys as sys;
use sys::errno;

use bitflags::bitflags;

use static_assertions::const_assert_eq;

use serde::{Deserialize, Serialize};

use std::{
    ffi::CString,
    fmt::Debug,
    os::{
        raw::{c_int, c_void},
        unix::io::RawFd,
    },
    time::Duration,
};

/// Prevent users from implementing the Socket & MutSocket trait.
mod private {
    pub trait Sealed {}
    impl Sealed for super::Push {}
    impl Sealed for super::Pull {}
    impl Sealed for super::PullConfig {}
    impl Sealed for super::Pair {}
    impl Sealed for super::Client {}
    impl Sealed for super::Server {}
    impl Sealed for super::Radio {}
    impl Sealed for super::Dish {}
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[doc(hidden)]
pub struct SocketConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    backlog: Option<i32>,
    connect_timeout: Option<Duration>,
    heartbeat_interval: Option<Duration>,
    heartbeat_timeout: Option<Duration>,
    heartbeat_ttl: Option<Duration>,
}

/// The set of shared socket configuration methods.
pub trait SharedConfig: private::Sealed {
    #[doc(hidden)]
    fn socket_config(&self) -> &SocketConfig;

    #[doc(hidden)]
    fn mut_socket_config(&mut self) -> &mut SocketConfig;

    fn connect(&mut self, endpoints: Vec<Endpoint>) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.connect = Some(endpoints);
        self
    }

    fn bind(&mut self, endpoints: Vec<Endpoint>) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.bind = Some(endpoints);
        self
    }

    fn backlog(&mut self, len: i32) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.backlog = Some(len);
        self
    }

    fn connect_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.connect_timeout = maybe_duration;
        self
    }

    fn heartbeat_interval(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.heartbeat_interval = maybe_duration;
        self
    }

    fn heartbeat_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.heartbeat_timeout = maybe_duration;
        self
    }

    fn heartbeat_ttl(&mut self, maybe_duration: Option<Duration>) -> &mut Self {
        let mut config = self.mut_socket_config();
        config.heartbeat_ttl = maybe_duration;
        self
    }

    fn apply(&self, socket: &mut Pull) -> Result<(), Error<()>> {
        let config = self.socket_config();

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
        impl SharedConfig for $name {
            #[doc(hidden)]
            fn socket_config(&self) -> &SocketConfig {
                &self.inner
            }

            #[doc(hidden)]
            fn mut_socket_config(&mut self) -> &mut SocketConfig {
                &mut self.inner
            }
        }
    };
}

fn connect(mut_sock_ptr: *mut c_void, c_str: CString) -> Result<(), Error<()>> {
    let rc = unsafe { sys::zmq_connect(mut_sock_ptr, c_str.as_ptr()) };

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

fn bind(mut_sock_ptr: *mut c_void, c_str: CString) -> Result<(), Error<()>> {
    let rc = unsafe { sys::zmq_bind(mut_sock_ptr, c_str.as_ptr()) };

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
    mut_sock_ptr: *mut c_void,
    c_str: CString,
) -> Result<(), Error<()>> {
    let rc = unsafe { sys::zmq_disconnect(mut_sock_ptr, c_str.as_ptr()) };

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

fn unbind(mut_sock_ptr: *mut c_void, c_str: CString) -> Result<(), Error<()>> {
    let rc = unsafe { sys::zmq_unbind(mut_sock_ptr, c_str.as_ptr()) };

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

/// The set of socket options shared among all socket types. We parametrize the
/// type of `self` so that this macro can be used for bot the `Socket` and `MutSocket`
/// trait.
macro_rules! impl_shared_socket_options {
    ($tyself:ty) => {
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
        fn connect<E>(self: $tyself, endpoint: E) -> Result<(), Error<()>>
        where
            E: AsRef<Endpoint>,
        {
            let c_str = CString::new(format!("{}", endpoint.as_ref())).unwrap();
            connect(self.mut_sock_ptr(), c_str)
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
        fn disconnect<E>(self: $tyself, endpoint: E) -> Result<(), Error<()>>
        where
            E: AsRef<Endpoint>,
        {
            let c_str = CString::new(format!("{}", endpoint.as_ref())).unwrap();
            disconnect(self.mut_sock_ptr(), c_str)
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
        fn bind<E>(self: $tyself, endpoint: E) -> Result<(), Error<()>>
        where
            E: AsRef<Endpoint>,
        {
            let c_str = CString::new(format!("{}", endpoint.as_ref())).unwrap();
            bind(self.mut_sock_ptr(), c_str)
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
        fn unbind<E>(self: $tyself, endpoint: E) -> Result<(), Error<()>>
        where
            E: AsRef<Endpoint>,
        {
            let c_str = CString::new(format!("{}", endpoint.as_ref())).unwrap();
            unbind(self.mut_sock_ptr(), c_str)
        }

        /// Retrieve the maximum length of the queue of outstanding peer connections.
        ///
        /// See `ZMQ_BLACKLOG` in [`zmq_getsockopt`].
        ///
        /// [`zmq_getsockopt`]: http://api.zeromq.org/master:zmq-getsockopt
        fn backlog(&self) -> Result<i32, Error<()>> {
            // This is safe the call does not actually mutate the socket.
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_scalar(mut_sock_ptr, SocketOption::Backlog)
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
        fn set_backlog(self: $tyself, value: i32) -> Result<(), Error<()>> {
            setsockopt_scalar(self.mut_sock_ptr(), SocketOption::Backlog, value)
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
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            let maybe_duration = getsockopt_duration(mut_sock_ptr, SocketOption::ConnectTimeout)?;
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
            self: $tyself,
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
                self.mut_sock_ptr(),
                SocketOption::ConnectTimeout,
                maybe_duration
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
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_scalar(mut_sock_ptr, SocketOption::FileDescriptor)
        }

        /// The interval between sending ZMTP heartbeats.
        fn heartbeat_interval(&self) -> Result<Option<Duration>, Error<()>> {
            // This is safe the call does not actually mutate the socket.
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_duration(mut_sock_ptr, SocketOption::HeartbeatInterval)
        }

        /// Sets the interval between sending ZMTP PINGs (aka. heartbeats).
        ///
        /// # Default Value
        /// `None`
        ///
        /// # Applicable Socket Type
        /// All (connection oriented transports)
        fn set_heartbeat_interval(
            self: $tyself,
            maybe_duration: Option<Duration>,
        ) -> Result<(), Error<()>> {
            setsockopt_duration(
                self.mut_sock_ptr(),
                SocketOption::HeartbeatInterval,
                maybe_duration
            )
        }

        /// How long to wait before timing-out a connection after sending a
        /// PING ZMTP command and not receiving any traffic.
        fn heartbeat_timeout(&self) -> Result<Option<Duration>, Error<()>> {
            // This is safe the call does not actually mutate the socket.
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_duration(mut_sock_ptr, SocketOption::HeartbeatTimeout)
        }

        /// How long to wait before timing-out a connection after sending a
        /// PING ZMTP command and not receiving any traffic.
        ///
        /// # Default Value
        /// `None`. If `heartbeat_interval` is set, then it uses the same value
        /// by default.
        fn set_heartbeat_timeout(
            self: $tyself,
            maybe_duration: Option<Duration>
            ) -> Result<(), Error<()>> {
            setsockopt_duration(
                self.mut_sock_ptr(),
                SocketOption::HeartbeatTimeout,
                maybe_duration
            )
        }

        /// The timeout on the remote peer for ZMTP heartbeats.
        /// If this option and `heartbeat_interval` is not `None` the remote
        /// side shall time out the connection if it does not receive any more
        /// traffic within the TTL period.
        fn heartbeat_ttl(&self) -> Result<Option<Duration>, Error<()>> {
            // This is safe the call does not actually mutate the socket.
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_duration(mut_sock_ptr, SocketOption::HeartbeatTtl)
        }

        /// Set timeout on the remote peer for ZMTP heartbeats.
        /// If this option and `heartbeat_interval` is not `None` the remote
        /// side shall time out the connection if it does not receive any more
        /// traffic within the TTL period.
        ///
        /// # Default value
        /// `None`
        fn set_heartbeat_ttl(
            self: $tyself,
            maybe_duration:
            Option<Duration>
        ) -> Result<(), Error<()>> {
            if let Some(ref duration) = maybe_duration {
                let ms = duration.as_millis();
                assert!(ms <= 6553599, "duration ms cannot exceed 6553599");
            }
            setsockopt_duration(
                self.mut_sock_ptr(),
                SocketOption::HeartbeatTtl,
                maybe_duration
            )
        }
    }
}

macro_rules! impl_send_socket_options {
    ($tyself:ty) => {
        /// The high water mark for outbound messages on the specified socket.
        ///
        /// The high water mark is a hard limit on the maximum number of
        /// outstanding messages ØMQ shall queue in memory.
        ///
        /// If this limit has been reached the socket shall enter the `mute state`.
        fn send_high_water_mark(&self) -> Result<Option<i32>, Error<()>> {
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            let limit = getsockopt_scalar(
                mut_sock_ptr,
                SocketOption::SendHighWaterMark,
            )?;

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
            self: $tyself,
            maybe_limit: Option<i32>
        ) -> Result<(), Error<()>> {
            match maybe_limit {
                Some(limit) => {
                    assert!(limit != 0, "high water mark cannot be zero");
                    setsockopt_scalar(
                        self.mut_sock_ptr(),
                        SocketOption::SendHighWaterMark,
                        limit
                    )
                }
                None => {
                    setsockopt_scalar(
                        self.mut_sock_ptr(),
                        SocketOption::SendHighWaterMark,
                        0
                    )
                }
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
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_duration(
                mut_sock_ptr,
                SocketOption::SendTimeout,
            )
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
            self: $tyself,
            maybe_duration: Option<Duration>,
        ) -> Result<(), Error<()>> {
            setsockopt_duration(
                self.mut_sock_ptr(),
                SocketOption::SendTimeout,
                maybe_duration,
            )
        }
    };
}

macro_rules! impl_recv_socket_options {
    ($tyself:ty) => {
        /// The high water mark for incoming messages on the specified socket.
        ///
        /// The high water mark is a hard limit on the maximum number of
        /// incoming messages ØMQ shall queue in memory.
        ///
        /// If this limit has been reached the socket shall enter the `mute state`.
        fn recv_high_water_mark(&self) -> Result<Option<i32>, Error<()>> {
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            let limit = getsockopt_scalar(
                mut_sock_ptr,
                SocketOption::RecvHighWaterMark,
            )?;

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
            self: $tyself,
            maybe_limit: Option<i32>
        ) -> Result<(), Error<()>> {
            match maybe_limit {
                Some(limit) => {
                    assert!(limit != 0, "high water mark cannot be zero");
                    setsockopt_scalar(
                        self.mut_sock_ptr(),
                        SocketOption::RecvHighWaterMark,
                        limit
                    )
                }
                None => {
                    setsockopt_scalar(
                        self.mut_sock_ptr(),
                        SocketOption::RecvHighWaterMark,
                        0
                    )
                }
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
            let mut_sock_ptr = self.sock_ptr() as *mut _;
            getsockopt_duration(
                mut_sock_ptr,
                SocketOption::RecvTimeout,
            )
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
            self: $tyself,
            maybe_duration: Option<Duration>,
        ) -> Result<(), Error<()>> {
            setsockopt_duration(
                self.mut_sock_ptr(),
                SocketOption::RecvTimeout,
                maybe_duration,
            )
        }
    };
}

/// Methods shared by all mutable (non thread-safe) sockets.
pub trait MutSocket: private::Sealed {
    #[doc(hidden)]
    fn sock_ptr(&self) -> *const c_void;

    #[doc(hidden)]
    fn mut_sock_ptr(&mut self) -> *mut c_void;

    impl_shared_socket_options!(&mut Self);
}

/// Methods shared by all thread-safe sockets.
pub trait Socket: private::Sealed {
    #[doc(hidden)]
    fn sock_ptr(&self) -> *const c_void;

    #[doc(hidden)]
    fn mut_sock_ptr(&self) -> *mut c_void;

    impl_shared_socket_options!(&Self);
}

bitflags! {
    /// A bitflag type used in socket `send` and `recv` method calls.
    pub struct Flags: c_int {
        /// [`Read more`](constant.NONE.html)
        const NONE = 0b00_000_000;
        /// [`Read more`](constant.NO_BLOCK.html)
        const NO_BLOCK = 0b00_000_001;
        /// [`Read more`](constant.MORE.html)
        const MORE = 0b00_000_010;
    }
}

/// Request default behavior for the operation.
///
/// The specific default behavior depends on the socket's
/// action while in [`mute state`].
///
/// [`mute state`]: trait.Socket.html#mute-state
pub const NONE: Flags = Flags::NONE;
/// Request non-blocking behavior for the operation.
pub const NO_BLOCK: Flags = Flags::NO_BLOCK;
/// Signals that the message is part of a multipart message.
pub const MORE: Flags = Flags::MORE;

const_assert_eq!(flags_none; Flags::NONE.bits, 0);
const_assert_eq!(flags_no_block; Flags::NO_BLOCK.bits, sys::ZMQ_NOBLOCK as c_int);
const_assert_eq!(flags_more; Flags::MORE.bits, sys::ZMQ_SNDMORE as c_int);

/// Attempts to send the pointed msg via the pointed socket. This function
/// does not drop the pointed msg on success.
///
/// # Usage Contract
/// ## Single part messages
/// * On success, the caller has to forfeit ownership of the `sys::zmq_msg_t`.
/// * On failure the caller can maintain ownership of the `sys::zmq_msg_t`.
///
/// ## Multipart messages
/// * If every `send` call in the multipart message succeed, the caller has to
///   forfeit ownership of the `sys::zmq_msg_t`.
/// * If any `send` call in the multipart message fails, the caller can maintain
///   ownership
unsafe fn send_no_drop(
    mut_sock_ptr: *mut c_void,
    mut_msg_ptr: *mut sys::zmq_msg_t,
    flags: Flags,
) -> Result<(), Error<()>> {
    let rc = sys::zmq_msg_send(mut_msg_ptr, mut_sock_ptr, flags.bits());

    if rc == -1 {
        let errno = sys::zmq_errno();
        let err = {
            match errno {
                errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
                errno::ENOTSUP => {
                    panic!("send is not supported by socket type")
                }
                errno::EINVAL => panic!(
                    "multipart messages are not supported by socket type"
                ),
                errno::EFSM => panic!(
                    "operation cannot be completed in current socket state"
                ),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::EFAULT => panic!("invalid message"),
                errno::EHOSTUNREACH => Error::new(ErrorKind::HostUnreachable),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn send_atomic(
    mut_sock_ptr: *mut c_void,
    mut msg: Msg,
    flags: Flags,
) -> Result<(), Error<Msg>> {
    assert!(!flags.contains(MORE));
    match unsafe { send_no_drop(mut_sock_ptr, msg.as_mut_ptr(), flags) } {
        Err(err) => {
            // The call failed so we give ownership of the message back to the caller.
            Err(Error::with_content(err.kind(), msg))
        }
        // The call succeeded, ØMQ now has ownership of the msg.
        Ok(_) => Ok(()),
    }
}

fn send_no_more(
    mut_sock_ptr: *mut c_void,
    mut msg: Msg,
    flags: Flags,
    buffer: &mut Vec<Msg>,
) -> Result<(), Error<Vec<Msg>>> {
    assert!(!flags.contains(MORE));
    if buffer.is_empty() {
        // Not part of a multipart message.
        match send_atomic(mut_sock_ptr, msg, flags) {
            Ok(_) => Ok(()),
            Err(mut err) => {
                let msg = err.take_content().unwrap();
                Err(Error::with_content(err.kind(), vec![msg]))
            }
        }
    } else {
        // The final part of a multipart message.
        match unsafe { send_no_drop(mut_sock_ptr, msg.as_mut_ptr(), flags) } {
            // The call failed so we give ownership back to the caller.
            Err(err) => {
                let mut drained: Vec<Msg> = buffer.drain(..).collect();
                drained.push(msg);
                assert!(buffer.is_empty());

                Err(Error::with_content(err.kind(), drained))
            }
            // The call succeeded, ØMQ now owns the messages in the buffer
            // so we drop them.
            Ok(_) => {
                buffer.clear();
                Ok(())
            }
        }
    }
}

fn send_more(
    mut_sock_ptr: *mut c_void,
    mut msg: Msg,
    flags: Flags,
    buffer: &mut Vec<Msg>,
) -> Result<(), Error<Vec<Msg>>> {
    assert!(flags.contains(MORE));

    let mut_msg_ptr = msg.as_mut_ptr();
    buffer.push(msg);

    match unsafe { send_no_drop(mut_sock_ptr, mut_msg_ptr, flags) } {
        // The call failed so we ownership of the messages in the buffer
        // back to the caller.
        Err(err) => {
            let drained = buffer.drain(..).collect();
            assert!(buffer.is_empty());

            Err(Error::with_content(err.kind(), drained))
        }
        // The call succeeded, but we keep ownership of the messages in the
        // buffer in case a future multipart call fails.
        Ok(_) => Ok(()),
    }
}

/// This function is use to provide the guarantee that the `send` and `send_parts`
/// methods of the `SendPart` trait cannot return `HostUnreachable`.
///
/// The only socket that could return in this case `HostUnreachable` is the `Router`,
/// which we dont support. This function is meant to detect regressions.
fn panic_on_host_unreachable<O, E>(result: &Result<O, Error<E>>)
where
    E: 'static + Send + Sync + Debug,
{
    if let Err(err) = result {
        if let ErrorKind::HostUnreachable = err.kind() {
            unreachable!()
        }
    }
}

fn send_part(
    mut_sock_ptr: *mut c_void,
    msg: Msg,
    flags: Flags,
    buffer: &mut Vec<Msg>,
) -> Result<(), Error<Vec<Msg>>> {
    let result = {
        if flags.contains(MORE) {
            send_more(mut_sock_ptr, msg, flags, buffer)
        } else {
            send_no_more(mut_sock_ptr, msg, flags, buffer)
        }
    };
    panic_on_host_unreachable(&result);

    result
}

/// Send messages parts in a mutable, non-thread-safe fashion.
///
/// Supports multipart messages.
pub trait SendPart: MutSocket {
    #[doc(hidden)]
    fn mut_buffer(&mut self) -> &mut Vec<Msg>;

    /// Send a message part from a socket.
    ///
    /// If the message is a `Msg`, `Vec<u8>` or a `String`, it is not copied.
    ///
    /// See [`zmq_msg_send`].
    ///
    /// # Success
    /// If the [`NONE`] flag was passed, the message, as well as all the previous
    /// multiparts messages (if any), were queued and now belong to ØMQ.
    ///
    /// If the [`MORE`] flag was passed, then success doesn't mean anything.
    ///
    /// # Error
    /// The message was not queued and its ownership, along with any previous
    /// multipart messages, is retured to the caller.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`NONE`]: constant.NONE.html
    /// [`MORE`]: constant.MORE.html
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn send<M>(&mut self, part: M, flags: Flags) -> Result<(), Error<Vec<Msg>>>
    where
        M: Into<Msg>,
    {
        let msg: Msg = part.into();
        send_part(self.mut_sock_ptr(), msg, flags, self.mut_buffer())
    }

    /// A convenience method that sends multipart message mutably from a socket.
    ///
    /// The `flags` are applied to each `send` call in the iterable.
    ///
    /// If the elements of the iterable are of the type `Msg`, `Vec<u8>` or `String`,
    /// they are not copied.
    ///
    /// # Error
    /// In case of an error, none of the messages are queued and their
    /// ownership is returned.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn send_parts<P, M>(
        &mut self,
        parts: P,
        flags: Flags,
    ) -> Result<(), Error<Vec<Msg>>>
    where
        P: IntoIterator<Item = M>,
        M: Into<Msg>,
    {
        let mut maybe_err: Option<Error<Vec<Msg>>> = None;
        let mut maybe_prev: Option<Msg> = None;
        let iter = parts.into_iter();
        // Since we consume the iterator, we might need to
        // store the messages somewhere if the loop errors.
        let mut maybe_buf: Option<Vec<Msg>> = None;
        // Keep track of the nb of elems left in the iterator
        // in case we have to alloc a buffer.
        let mut elems_left = iter.size_hint().0;

        for part in iter {
            let msg: Msg = part.into();
            if maybe_err.is_some() {
                maybe_buf.as_mut().unwrap().push(msg);
            } else {
                let maybe_msg = maybe_prev.take();
                elems_left -= 1;
                if let Some(msg) = maybe_msg {
                    if let Err(err) = send_part(
                        self.mut_sock_ptr(),
                        msg,
                        flags | MORE,
                        self.mut_buffer(),
                    ) {
                        maybe_buf = Some(Vec::with_capacity(elems_left));
                        maybe_err = Some(err);
                    }
                }
                maybe_prev = Some(msg);
            }
        }

        if let Some(mut err) = maybe_err {
            // Loop errored, so we return ownership of the messages to
            // the caller.
            let mut content = err.take_content().unwrap();
            content.extend(maybe_buf.unwrap());

            Err(Error::with_content(err.kind(), content))
        } else if let Some(msg) = maybe_prev {
            send_part(self.mut_sock_ptr(), msg, flags, self.mut_buffer())
        } else {
            // Empty iterator case.
            Ok(())
        }
    }

    impl_send_socket_options!(&mut Self);
}

/// Send messages in a thread-safe fashion.
///
/// Does not support multipart messages.
pub trait SendAtomic: Socket + Send + Sync {
    /// Send a message immutably from a socket.
    ///
    /// If the message is a `Msg`, `Vec<u8>` or a `String`, it is not copied.
    ///
    /// See [`zmq_msg_send`].
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
    fn send<M>(&self, sendable: M, flags: Flags) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        assert!(!flags.contains(MORE), "MORE flag is not allowed");
        let msg: Msg = sendable.into();
        send_atomic(self.mut_sock_ptr(), msg, flags)
    }

    impl_send_socket_options!(&Self);
}

fn recv(
    mut_sock_ptr: *mut c_void,
    msg: &mut Msg,
    flags: Flags,
) -> Result<(), Error<()>> {
    let rc = unsafe {
        sys::zmq_msg_recv(msg.as_mut_ptr(), mut_sock_ptr, flags.bits())
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

fn recv_parts(
    mut_sock_ptr: *mut c_void,
    queue: &mut Vec<Msg>,
    flags: Flags,
) -> Result<(), Error<()>> {
    for msg in queue.iter_mut() {
        recv(mut_sock_ptr, msg, flags)?;
        if !msg.more() {
            return Ok(());
        }
    }

    loop {
        let mut msg = Msg::new();
        recv(mut_sock_ptr, &mut msg, flags)?;

        let more = msg.more();
        queue.push(msg);

        if !more {
            return Ok(());
        }
    }
}

/// Receive messages parts in a mutable, non-thread-safe fashion.
///
/// Supports multipart messages.
pub trait RecvPart: MutSocket {
    /// Receive a message part mutably from a socket.
    ///
    /// See [`zmq_msg_recv`].
    ///
    /// # Error
    /// In case of an error the message that failed to be received is
    /// not lost, unless the `Ctx` was terminated.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`zmq_msg_recv`]: http://api.zeromq.org/master:zmq-msg-recv
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn recv(&mut self, msg: &mut Msg, flags: Flags) -> Result<(), Error<()>> {
        assert!(!flags.contains(MORE), "MORE flag not allowed");
        recv(self.mut_sock_ptr(), msg, flags)
    }

    /// A convenience function with the same properties as [`recv`] but
    /// that returns an allocated a [`Msg`].
    ///
    /// [`recv`]: #method.recv
    /// [`Msg`]: ../msg/struct.Msg.html
    fn recv_msg(&mut self, flags: Flags) -> Result<Msg, Error<()>> {
        let mut msg = Msg::new();
        self.recv(&mut msg, flags)?;

        Ok(msg)
    }

    /// A convenience method that receives a multipart message mutably
    /// from a socket.
    ///
    /// The `flags` are applied to each `recv` call in the iterable.
    ///
    /// This might allocate to the `queue`.
    ///
    /// # Error
    /// In case of an error the multipart message that failed to be
    /// received is not lost, unless the `Ctx` was terminated.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn recv_parts(
        &mut self,
        queue: &mut Vec<Msg>,
        flags: Flags,
    ) -> Result<(), Error<()>> {
        assert!(!flags.contains(MORE), "MORE flag not allowed");
        recv_parts(self.mut_sock_ptr(), queue, flags)
    }

    /// A convenience function with the same properties as [`recv_parts`] but
    /// that returns an allocated a `Vec<Msg>`.
    ///
    /// [`recv_parts`]: #method.recv_parts
    /// [`Msg`]: ../msg/struct.Msg.html
    fn recv_msg_parts(&mut self, flags: Flags) -> Result<Vec<Msg>, Error<()>> {
        let mut queue = Vec::new();
        self.recv_parts(&mut queue, flags)?;

        Ok(queue)
    }

    impl_recv_socket_options!(&mut Self);
}

/// Receive atomic messages in an immutable, thread-safe fashion.
///
/// Does not support multipart messages.
pub trait RecvAtomic: Socket + Send + Sync {
    /// Receive a message immutably from a socket.
    ///
    /// See [`zmq_msg_recv`].
    ///
    /// # Error
    /// In case of an error the message that failed to be received is
    /// not lost, unless the `Ctx` was terminated.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`]
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    ///
    /// [`zmq_msg_recv`]: http://api.zeromq.org/master:zmq-msg-recv
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn recv(&self, msg: &mut Msg, flags: Flags) -> Result<(), Error<()>> {
        assert!(!flags.contains(MORE), "MORE flag not allowed");
        // This is safe since this socket type is thread safe.
        recv(self.mut_sock_ptr(), msg, flags)
    }

    /// A convenience function that allocates a [`Msg`] with the same properties as [`recv`].
    ///
    /// [`recv`]: #method.recv
    /// [`Msg`]: ../msg/struct.Msg.html
    fn recv_msg(&self, flags: Flags) -> Result<Msg, Error<()>> {
        let mut msg = Msg::new();
        self.recv(&mut msg, flags)?;

        Ok(msg)
    }

    impl_recv_socket_options!(&Self);
}

enum SocketType {
    Push,
    Pull,
    Pair,
    Client,
    Server,
    Radio,
    Dish,
}

impl Into<c_int> for SocketType {
    fn into(self) -> c_int {
        match self {
            SocketType::Push => sys::ZMQ_PUSH as c_int,
            SocketType::Pull => sys::ZMQ_PULL as c_int,
            SocketType::Pair => sys::ZMQ_PAIR as c_int,
            SocketType::Client => sys::ZMQ_CLIENT as c_int,
            SocketType::Server => sys::ZMQ_SERVER as c_int,
            SocketType::Radio => sys::ZMQ_RADIO as c_int,
            SocketType::Dish => sys::ZMQ_DISH as c_int,
        }
    }
}

struct RawSocket {
    ctx: Ctx,
    socket: *mut c_void,
}

impl RawSocket {
    fn new(sock_type: SocketType) -> Result<Self, Error<()>> {
        let ctx = Ctx::global().clone();

        Self::with_ctx(sock_type, ctx)
    }

    fn with_ctx(sock_type: SocketType, ctx: Ctx) -> Result<Self, Error<()>> {
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

        assert_eq!(rc, 0, "invalid socket");
    }
}

/// Implement the shared methods for a buffered socket.
macro_rules! impl_shared_buf_methods {
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
            let inner = RawSocket::new(SocketType::$name)?;

            Ok(Self {
                inner,
                buffer: vec![],
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
            let inner = RawSocket::with_ctx(SocketType::$name, ctx)?;

            Ok(Self {
                inner,
                buffer: vec![],
            })
        }

        /// Returns a reference to the context of the socket.
        pub fn ctx(&self) -> &Ctx {
            &self.inner.ctx
        }
    };

    ($name:tt) => {
        impl_shared_buf_methods!($name, stringify!($name));
    };
}

/// Implement the methods for an unbuffered socket.
macro_rules! impl_shared_methods {
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
                let inner = RawSocket::new(SocketType::$name)?;

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
                let inner = RawSocket::with_ctx(SocketType::$name, ctx)?;

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
        impl_shared_methods!($name, stringify!($name));
    };
}

/// Implement the MutSocket trait.
macro_rules! impl_mut_socket_trait {
    ($name:ident) => {
        impl MutSocket for $name {
            fn sock_ptr(&self) -> *const c_void {
                self.inner.socket
            }

            fn mut_sock_ptr(&mut self) -> *mut c_void {
                self.inner.socket
            }
        }
    };
}

/// Implement the Socket trait.
macro_rules! impl_socket_trait {
    ($name:ident) => {
        impl Socket for $name {
            fn sock_ptr(&self) -> *const c_void {
                self.inner.socket
            }

            // This is safe since this socket is thread safe.
            fn mut_sock_ptr(&self) -> *mut c_void {
                self.inner.socket as *mut _
            }
        }
    };
}

/// A `Push` socket is used by a pipeline node to send messages to
/// downstream pipeline nodes. Messages are round-robined to all connected
/// downstream nodes.
///
/// # Use case
/// The `Push` - [`Pull`] pattern provide the best message throughtput at the
/// expense of usability.
///
/// Indeed, since messaging is strictly unidirectionally, it does not provide
/// the ability to synchronize the producer with the workers.
/// This can be problematic because workers won't all connect to the producer
/// at the same time. Thus, a worker that connected first might receive more
/// messages than the others, until equilibrium is eventually reached.
/// Using a lower `recv_high_water_mark` on the workers can help mitigate this
/// effect.
///
/// # Mute state
/// When a `Push` socket enters the mute state due to having reached the high
/// water mark for all downstream nodes, or if there are no downstream nodes at
/// all, then any `send` operations on the socket shall block until the mute
/// state ends or at least one downstream node becomes available for sending;
/// messages are not discarded.
///
/// # Usage Example
///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::prelude::*;
///
/// let endpoint: Endpoint = "inproc://test".parse()?;
///
/// // Let's fire up our sockets. We have a task ventilator as well
/// // two workers.
/// let mut ventilator = Push::new()?;
/// let mut first = Pull::new()?;
/// let mut second = Pull::new()?;
///
/// // All of these sockets are currently in mute state since they
/// // don't have any peers. Let's fix that.
/// ventilator.bind(&endpoint)?;
/// first.connect(&endpoint)?;
/// second.connect(endpoint)?;
///
/// // Lets illustrate the synchronization problem. We distribute some
/// // tasks downstream.
/// ventilator.send_parts(vec!["one", "two", "three"], NONE)?;
///
/// // The first worker connected first so that, while the second worker was
/// // still connected, the task ventilator was able to send all its task
/// // downstream.
/// let mut msg = first.recv_msg(NONE)?;
/// assert_eq!("one", msg.to_str()?);
/// first.recv(&mut msg, NONE)?;
/// assert_eq!("two", msg.to_str()?);
/// first.recv(&mut msg, NONE)?;
/// assert_eq!("three", msg.to_str()?);
/// // Indeed the second worker did not receive any work.
/// let err = second.recv(&mut msg, NO_BLOCK).unwrap_err();
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Pull`]               |
/// | Direction                 | Unidirectional         |
/// | Pattern                   | Send only              |
/// | Incoming routing strategy | N/A                    |
/// | Outgoing routing strategy | Round-robin            |
/// | Action in mute state      | Block                  |
///
/// [`Pull`]: struct.Pull.html
pub struct Push {
    inner: RawSocket,
    buffer: Vec<Msg>,
}

impl Push {
    impl_shared_buf_methods!(Push);
}

impl_mut_socket_trait!(Push);

#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PullConfig {
    #[serde(flatten)]
    inner: SocketConfig,
}

impl PullConfig {
    pub fn build(&self) -> Result<Pull, Error<()>> {
        let mut pull = Pull::new()?;
        self.apply(&mut pull)?;

        Ok(pull)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Pull, Error<()>> {
        let mut pull = Pull::with_ctx(ctx)?;
        self.apply(&mut pull)?;

        Ok(pull)
    }
}

impl_config_trait!(PullConfig);

impl SendPart for Push {
    fn mut_buffer(&mut self) -> &mut Vec<Msg> {
        &mut self.buffer
    }
}

/// A `Pull` socket is used by a pipeline node to receive messages from
/// upstream pipeline nodes. Messages are fair-queued from among all connected
/// upstream nodes.
///
/// # Use Case
/// [`Read more here`](struct.Push.html#use-case).
///
/// # Mute state
/// When a `Pull` socket enters the mute state due to having reached the high
/// water mark for all upstream nodes, or if there are no upstream nodes at
/// all, then any `recv` operations on the socket shall block until the mute
/// state ends or at least one upstream node becomes available for receiving.
///
///# Usage Example
/// [`Read more here`](struct.Push.html#usage-example).
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Push`]               |
/// | Direction                 | Unidirectional         |
/// | Pattern                   | Receive only           |
/// | Incoming routing strategy | Fair-queued            |
/// | Outgoing routing strategy | N/A                    |
/// | Action in mute state      | Block                  |
pub struct Pull {
    inner: RawSocket,
}

impl Pull {
    impl_shared_methods!(Pull);
}

impl_mut_socket_trait!(Pull);

impl RecvPart for Pull {}

/// A socket of can only be connected to a single peer at any one time.
///
/// No message routing or filtering is performed on messages sent. The `Pair`
/// socket is designed for inter-thread communication across the
/// inproc transport and do not implement functionality such as
/// auto-reconnection.
///
/// # Use case
/// The `Pair` socket should be used to communicate accross thread boundaries
/// within the same process via the `inproc` transport.
///
/// # Mute State
/// When a `Pair` socket enters the mute state due to having reached the
/// high water mark for the connected peer, or if no peer is connected,
/// then any `send` operations on the socket shall block until the peer
/// becomes available for sending; messages are not discarded.
///
/// # Basic Usage Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::prelude::*;
/// use std::{thread, time::Duration};
///
/// let endpoint: Endpoint = "inproc://test".parse()?;
///
/// // We set `no_linger` so that the sockets disconnect `almost` instantly.
/// Ctx::global().set_no_linger(true);
///
/// let mut first = Pair::new()?;
/// let mut second = Pair::new()?;
///
/// // Both sockets are in mute state since they don't have any peers.
/// // While in mute state, a `Pair` socket blocks. Let's remedy that.
/// first.bind(&endpoint)?;
/// second.connect(&endpoint)?;
///
/// // Now both sockets have a peer so they won't block anymore.
/// // We can send single part messages...
/// first.send("one", NONE)?;
/// let mut msg = second.recv_msg(NONE)?;
/// assert_eq!("one", msg.to_str()?);
///
/// // ...as well as multipart messages.
/// second.send("cya", MORE)?;
/// second.send("l8er", NONE)?;
///
/// let parts = first.recv_msg_parts(NONE)?;
/// let strings: Vec<&str> = parts.iter().filter_map(|x| x.to_str().ok()).collect();
/// assert_eq!(["cya", "l8er"], strings.as_slice());
///
/// // Note that ØMQ guarantees that peers shall receive either all
/// // message parts or none at all. Therefore, this would block:
/// first.send("time", MORE)?;
/// let err = second.recv(&mut msg, NO_BLOCK).unwrap_err();
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// // Now lets disconnect one of the sockets.
/// second.disconnect(endpoint)?;
/// thread::sleep(Duration::from_millis(1)); // 'almost' instantly
///
/// // The first socket is now in mute state, so the next call would block.
/// let mut err = first.send("is money", NO_BLOCK).unwrap_err();
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// // We can get back the messages that weren't sent.
/// let parts = err.take_content().unwrap();
/// let strings: Vec<&str> = parts.iter().filter_map(|x| x.to_str().ok()).collect();
/// assert_eq!(["time", "is money"], strings.as_slice());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value         |
/// |:-------------------------:|:-------------:|
/// | Compatible peer           | ZMQ_PAIR      |
/// | Direction                 | Bidirectional |
/// | Pattern                   | Unrestricted  |
/// | Incoming routing strategy | N/A           |
/// | Outgoing routing strategy | N/A           |
/// | Action in mute state      | Block         |
pub struct Pair {
    inner: RawSocket,
    buffer: Vec<Msg>,
}

impl Pair {
    impl_shared_buf_methods!(Pair);
}

impl_mut_socket_trait!(Pair);

impl SendPart for Pair {
    fn mut_buffer(&mut self) -> &mut Vec<Msg> {
        &mut self.buffer
    }
}

impl RecvPart for Pair {}

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
/// second.connect(endpoint)?;
///
/// // Lets do the whole request-reply thing.
/// first.send("request", NONE)?;
///
/// let mut msg = second.recv_msg(NONE)?;
/// assert_eq!("request", msg.to_str()?);
///
/// second.send("reply", NONE)?;
///
/// first.recv(&mut msg, NONE)?;
/// assert_eq!("reply", msg.to_str()?);
///
/// // We can send as many replies as we want. We don't need to follow
/// // a strict one request equals one reply pattern.
/// second.send("another reply", NONE)?;
///
/// first.recv(&mut msg, NONE)?;
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
pub struct Client {
    inner: RawSocket,
}

impl Client {
    impl_shared_methods!(Client);
}

impl_socket_trait!(Client);

impl SendAtomic for Client {}
impl RecvAtomic for Client {}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

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
/// server.bind(endpoint)?;
///
/// // The client initiates the conversation so it is assigned a `routing_id`.
/// client.send("request", NONE)?;
/// let msg = server.recv_msg(NONE)?;
/// assert_eq!("request", msg.to_str()?);
/// let routing_id = msg.routing_id().expect("no routing id");
///
/// // Using this `routing_id`, we can now route as many replies as we
/// // want to the client.
/// let mut msg: Msg = "reply 1".into();
/// msg.set_routing_id(routing_id);
/// server.send(msg, NONE)?;
/// let mut msg: Msg = "reply 2".into();
/// msg.set_routing_id(routing_id);
/// server.send(msg, NONE)?;
///
/// // The `routing_id` is discarted when the message is sent to the client.
/// let mut msg = client.recv_msg(NONE)?;
/// assert_eq!("reply 1", msg.to_str()?);
/// assert!(msg.routing_id().is_none());
/// client.recv(&mut msg, NONE)?;
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
pub struct Server {
    inner: RawSocket,
}

impl Server {
    impl_shared_methods!(Server);
}

impl_socket_trait!(Server);

impl SendAtomic for Server {}
impl RecvAtomic for Server {}

unsafe impl Send for Server {}
unsafe impl Sync for Server {}

/// A `Radio` socket is used by a publisher to distribute data to [`Dish`]
/// sockets. Each message belong to a group specified with [`set_group`].
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
/// let addr: Endpoint = "inproc://test".parse().unwrap();
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
/// radio.bind(&addr)?;
/// first.connect(&addr)?;
/// second.connect(addr)?;
///
/// // Each dish will only receive messages from that group.
/// first.join("first group")?;
/// second.join("second group")?;
///
/// // Lets publish some messages to subscribers.
/// let mut msg: Msg = "first msg".into();
/// msg.set_group("first group")?;
/// radio.send(msg, NONE)?;
/// let mut msg: Msg = "second msg".into();
/// msg.set_group("second group")?;
/// radio.send(msg, NONE)?;
///
/// // Lets receive the publisher's messages.
/// let mut msg = first.recv_msg(NONE)?;
/// assert_eq!("first msg", msg.to_str().unwrap());
/// let err = first.recv(&mut msg, NO_BLOCK).unwrap_err();
/// // Only the message from the first group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// second.recv(&mut msg, NONE)?;
/// assert_eq!("second msg", msg.to_str().unwrap());
/// let err = first.recv(&mut msg, NO_BLOCK).unwrap_err();
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
pub struct Radio {
    inner: RawSocket,
}

impl Radio {
    impl_shared_methods!(Radio);

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> Result<bool, Error<()>> {
        getsockopt_bool(self.mut_sock_ptr(), SocketOption::NoDrop)
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
        setsockopt_bool(self.mut_sock_ptr(), SocketOption::NoDrop, enabled)
    }
}

impl_socket_trait!(Radio);

impl SendAtomic for Radio {}

unsafe impl Send for Radio {}
unsafe impl Sync for Radio {}

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
pub struct Dish {
    inner: RawSocket,
}

impl Dish {
    impl_shared_methods!(Dish);

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
        let rc = unsafe { sys::zmq_join(self.mut_sock_ptr(), c_str.as_ptr()) };

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
        let rc = unsafe { sys::zmq_leave(self.mut_sock_ptr(), c_str.as_ptr()) };

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

impl RecvAtomic for Dish {}

unsafe impl Send for Dish {}
unsafe impl Sync for Dish {}

#[cfg(test)]
mod test {}
