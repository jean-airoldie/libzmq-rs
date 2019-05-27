use crate::{addr::Endpoint, auth::*, core::sockopt::*, error::*, Ctx};
use libzmq_sys as sys;
use sys::errno;

use log::error;

use std::{
    ffi::CString,
    os::raw::{c_int, c_void},
    sync::Mutex,
    time::Duration,
};

const MAX_HB_TTL: i64 = 6_553_599;

#[doc(hidden)]
pub trait GetRawSocket: super::private::Sealed {
    fn raw_socket(&self) -> &RawSocket;
}

pub(crate) enum RawSocketType {
    Client = sys::ZMQ_CLIENT as isize,
    Server = sys::ZMQ_SERVER as isize,
    Radio = sys::ZMQ_RADIO as isize,
    Dish = sys::ZMQ_DISH as isize,
    Dealer = sys::ZMQ_DEALER as isize,
    Router = sys::ZMQ_ROUTER as isize,
    Pair = sys::ZMQ_PAIR as isize,
    Sub = sys::ZMQ_SUB as isize,
    Scatter = sys::ZMQ_SCATTER as isize,
    Gather = sys::ZMQ_GATHER as isize,
}

impl From<RawSocketType> for c_int {
    fn from(r: RawSocketType) -> c_int {
        match r {
            RawSocketType::Client => RawSocketType::Client as c_int,
            RawSocketType::Server => RawSocketType::Server as c_int,
            RawSocketType::Radio => RawSocketType::Radio as c_int,
            RawSocketType::Dish => RawSocketType::Dish as c_int,
            RawSocketType::Dealer => RawSocketType::Dealer as c_int,
            RawSocketType::Router => RawSocketType::Router as c_int,
            RawSocketType::Pair => RawSocketType::Pair as c_int,
            RawSocketType::Sub => RawSocketType::Sub as c_int,
            RawSocketType::Scatter => RawSocketType::Scatter as c_int,
            RawSocketType::Gather => RawSocketType::Gather as c_int,
        }
    }
}

fn connect(socket_ptr: *mut c_void, c_string: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_connect(socket_ptr, c_string.as_ptr()) };

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

fn bind(socket_ptr: *mut c_void, c_string: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_bind(socket_ptr, c_string.as_ptr()) };

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

fn disconnect(socket_ptr: *mut c_void, c_string: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_disconnect(socket_ptr, c_string.as_ptr()) };

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

/// This socket may or may not be thread safe depending on the `RawSocketType`.
/// We prevent that it is always thread-safe and let the wrapping types decide.
#[derive(Debug)]
#[doc(hidden)]
pub struct RawSocket {
    socket_mut_ptr: *mut c_void,
    ctx: Ctx,
    mechanism: Mutex<Mechanism>,
}

impl RawSocket {
    pub(crate) fn new(sock_type: RawSocketType) -> Result<Self, Error> {
        let ctx = Ctx::global().clone();
        Self::with_ctx(sock_type, ctx)
    }

    pub(crate) fn with_ctx<C>(
        sock_type: RawSocketType,
        ctx: C,
    ) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx = ctx.into();
        let socket_mut_ptr =
            unsafe { sys::zmq_socket(ctx.as_ptr(), sock_type.into()) };

        if socket_mut_ptr.is_null() {
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
            // Set ZAP domain handling to strictly adhere the RFC.
            // This will eventually be enabled by default by ØMQ.
            setsockopt_bool(socket_mut_ptr, SocketOption::EnforceDomain, true)?;
            // We hardset the same domain name for each sockets because I don't
            // see any use cases for them.
            setsockopt_str(
                socket_mut_ptr,
                SocketOption::ZapDomain,
                Some("global"),
            )?;

            Ok(Self {
                ctx,
                socket_mut_ptr,
                mechanism: Mutex::default(),
            })
        }
    }

    pub(crate) fn connect(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        connect(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn disconnect(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        disconnect(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn bind(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        bind(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn unbind(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        unbind(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    /// This is safe since the pointed socket is thread safe.
    pub(crate) fn as_mut_ptr(&self) -> *mut c_void {
        self.socket_mut_ptr
    }

    pub(crate) fn mechanism(&self) -> &Mutex<Mechanism> {
        &self.mechanism
    }

    pub(crate) fn last_endpoint(&self) -> Result<Option<Endpoint>, Error> {
        let maybe =
            getsockopt_string(self.as_mut_ptr(), SocketOption::LastEndpoint)?;

        Ok(maybe.map(|s| Endpoint::from_zmq(s.as_str())))
    }

    pub(crate) fn heartbeat_interval(&self) -> Result<Duration, Error> {
        getsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::HeartbeatInterval,
            -1,
        )
        .map(Option::unwrap)
    }

    pub(crate) fn set_heartbeat_interval(
        &self,
        duration: Duration,
    ) -> Result<(), Error> {
        setsockopt_duration(
            self.as_mut_ptr(),
            SocketOption::HeartbeatInterval,
            duration,
        )
    }

    pub(crate) fn heartbeat_timeout(&self) -> Result<Duration, Error> {
        getsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::HeartbeatTimeout,
            -1,
        )
        .map(Option::unwrap)
    }

    pub(crate) fn set_heartbeat_timeout(
        &self,
        duration: Duration,
    ) -> Result<(), Error> {
        setsockopt_duration(
            self.as_mut_ptr(),
            SocketOption::HeartbeatTimeout,
            duration,
        )
    }

    pub(crate) fn heartbeat_ttl(&self) -> Result<Duration, Error> {
        getsockopt_duration(self.as_mut_ptr(), SocketOption::HeartbeatTtl)
    }

    pub(crate) fn set_heartbeat_ttl(
        &self,
        duration: Duration,
    ) -> Result<(), Error> {
        let ms = duration.as_millis();
        if ms > MAX_HB_TTL as u128 {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "duration ms cannot exceed 6553599",
            }));
        }
        setsockopt_duration(
            self.as_mut_ptr(),
            SocketOption::HeartbeatTtl,
            duration,
        )
    }

    pub(crate) fn linger(&self) -> Result<Option<Duration>, Error> {
        getsockopt_option_duration(self.as_mut_ptr(), SocketOption::Linger, -1)
    }

    pub(crate) fn set_linger(
        &self,
        maybe: Option<Duration>,
    ) -> Result<(), Error> {
        setsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::Linger,
            maybe,
            -1,
        )
    }

    pub(crate) fn set_username(
        &self,
        maybe: Option<&str>,
    ) -> Result<(), Error> {
        setsockopt_str(self.as_mut_ptr(), SocketOption::PlainUsername, maybe)
    }

    pub(crate) fn set_password(
        &self,
        maybe: Option<&str>,
    ) -> Result<(), Error> {
        setsockopt_str(self.as_mut_ptr(), SocketOption::PlainPassword, maybe)
    }

    pub(crate) fn set_plain_server(&self, cond: bool) -> Result<(), Error> {
        setsockopt_bool(self.as_mut_ptr(), SocketOption::PlainServer, cond)
    }

    pub(crate) fn recv_high_water_mark(&self) -> Result<Option<i32>, Error> {
        getsockopt_option_scalar(
            self.as_mut_ptr(),
            SocketOption::RecvHighWaterMark,
            0,
        )
    }

    pub(crate) fn set_recv_high_water_mark(
        &self,
        maybe: Option<i32>,
    ) -> Result<(), Error> {
        if let Some(hwm) = maybe {
            assert!(hwm != 0, "high water mark cannot be zero");
        }

        setsockopt_option_scalar(
            self.as_mut_ptr(),
            SocketOption::RecvHighWaterMark,
            maybe,
            0,
        )
    }

    pub(crate) fn recv_timeout(&self) -> Result<Option<Duration>, Error> {
        getsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::RecvTimeout,
            -1,
        )
    }

    pub(crate) fn set_recv_timeout(
        &self,
        maybe: Option<Duration>,
    ) -> Result<(), Error> {
        setsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::RecvTimeout,
            maybe,
            -1,
        )
    }

    pub(crate) fn send_high_water_mark(&self) -> Result<Option<i32>, Error> {
        getsockopt_option_scalar(
            self.as_mut_ptr(),
            SocketOption::SendHighWaterMark,
            0,
        )
    }

    pub(crate) fn set_send_high_water_mark(
        &self,
        maybe: Option<i32>,
    ) -> Result<(), Error> {
        if let Some(hwm) = maybe {
            assert!(hwm != 0, "high water mark cannot be zero");
        }

        setsockopt_option_scalar(
            self.as_mut_ptr(),
            SocketOption::SendHighWaterMark,
            maybe,
            0,
        )
    }

    pub(crate) fn send_timeout(&self) -> Result<Option<Duration>, Error> {
        getsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::SendTimeout,
            -1,
        )
    }

    pub(crate) fn set_send_timeout(
        &self,
        maybe: Option<Duration>,
    ) -> Result<(), Error> {
        setsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::SendTimeout,
            maybe,
            -1,
        )
    }

    pub(crate) fn no_drop(&self) -> Result<bool, Error> {
        getsockopt_bool(self.as_mut_ptr(), SocketOption::NoDrop)
    }

    pub(crate) fn set_no_drop(&self, enabled: bool) -> Result<(), Error> {
        setsockopt_bool(self.as_mut_ptr(), SocketOption::NoDrop, enabled)
    }

    pub(crate) fn set_curve_public_key(
        &self,
        key: Option<&BinCurveKey>,
    ) -> Result<(), Error> {
        let key = key.map(BinCurveKey::as_bytes);
        setsockopt_bytes(self.as_mut_ptr(), SocketOption::CurvePublicKey, key)
    }

    pub(crate) fn set_curve_secret_key(
        &self,
        key: Option<&BinCurveKey>,
    ) -> Result<(), Error> {
        let key = key.map(BinCurveKey::as_bytes);
        setsockopt_bytes(self.as_mut_ptr(), SocketOption::CurveSecretKey, key)
    }

    pub(crate) fn set_curve_server(&self, enabled: bool) -> Result<(), Error> {
        setsockopt_bool(self.as_mut_ptr(), SocketOption::CurveServer, enabled)
    }

    pub(crate) fn set_curve_server_key(
        &self,
        key: Option<&BinCurveKey>,
    ) -> Result<(), Error> {
        let key = key.map(BinCurveKey::as_bytes);
        setsockopt_bytes(self.as_mut_ptr(), SocketOption::CurveServerKey, key)
    }
}

impl PartialEq for RawSocket {
    fn eq(&self, other: &RawSocket) -> bool {
        self.socket_mut_ptr == other.socket_mut_ptr
    }
}

impl Eq for RawSocket {}

impl Drop for RawSocket {
    /// Close the ØMQ socket.
    ///
    /// See [`zmq_close`].
    ///
    /// [`zmq_close`]: http://api.zeromq.org/master:zmq-close
    fn drop(&mut self) {
        let rc = unsafe { sys::zmq_close(self.socket_mut_ptr) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            error!("error while dropping socket: {}", msg_from_errno(errno));
        }
    }
}
