use crate::{
    addr::Endpoint,
    auth::*,
    core::sockopt::*,
    core::{Heartbeat, Period},
    error::*,
    Ctx, CtxHandle,
};

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
pub trait AsRawSocket: super::private::Sealed {
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
        let err = match errno {
            errno::EINVAL => {
                panic!("invalid endpoint : {}", c_string.to_string_lossy())
            }
            errno::EPROTONOSUPPORT => {
                Error::new(ErrorKind::InvalidInput("transport not supported"))
            }
            errno::ENOCOMPATPROTO => {
                Error::new(ErrorKind::InvalidInput("transport incompatible"))
            }
            errno::ETERM => Error::new(ErrorKind::CtxInvalid),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EMTHREAD => panic!("no i/o thread available"),
            _ => panic!(msg_from_errno(errno)),
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
        let err = match errno {
            errno::EINVAL => {
                panic!("invalid endpoint : {}", c_string.to_string_lossy())
            }
            errno::EPROTONOSUPPORT => {
                Error::new(ErrorKind::InvalidInput("transport not supported"))
            }
            errno::ENOCOMPATPROTO => {
                Error::new(ErrorKind::InvalidInput("transport incompatible"))
            }
            errno::EADDRINUSE => Error::new(ErrorKind::AddrInUse),
            errno::EADDRNOTAVAIL => Error::new(ErrorKind::AddrNotAvailable),
            errno::ENODEV => Error::new(ErrorKind::AddrNotAvailable),
            errno::ETERM => Error::new(ErrorKind::CtxInvalid),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EMTHREAD => panic!("no i/o thread available"),
            _ => panic!(msg_from_errno(errno)),
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
        let err = match errno {
            errno::EINVAL => {
                panic!("invalid endpoint : {}", c_string.to_string_lossy())
            }
            errno::ETERM => Error::new(ErrorKind::CtxInvalid),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::ENOENT => {
                Error::new(ErrorKind::NotFound("endpoint was not in use"))
            }
            _ => panic!(msg_from_errno(errno)),
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn unbind(socket_ptr: *mut c_void, c_string: CString) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_unbind(socket_ptr, c_string.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = match errno {
            errno::EINVAL => {
                panic!("invalid endpoint : {}", c_string.to_string_lossy())
            }
            errno::ETERM => Error::new(ErrorKind::CtxInvalid),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::ENOENT => {
                Error::new(ErrorKind::NotFound("endpoint was not bound to"))
            }
            _ => panic!(msg_from_errno(errno)),
        };

        Err(err)
    } else {
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct SocketHandle(*mut c_void);

impl SocketHandle {
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct RawSocket {
    ptr: *mut c_void,
    ctx: CtxHandle,
    mechanism: Mutex<Mechanism>,
    heartbeat: Mutex<Option<Heartbeat>>,
}

impl RawSocket {
    pub(crate) fn new(sock_type: RawSocketType) -> Result<Self, Error> {
        let handle = Ctx::global();
        Self::with_ctx(sock_type, handle)
    }

    pub(crate) fn with_ctx(
        sock_type: RawSocketType,
        ctx: CtxHandle,
    ) -> Result<Self, Error> {
        let ptr =
            unsafe { sys::zmq_socket(ctx.as_ptr(), sock_type.into()) };

        if ptr.is_null() {
            let errno = unsafe { sys::zmq_errno() };
            let err = match errno {
                errno::EINVAL => panic!("invalid socket type"),
                // The context associated with the handle was terminated.
                errno::EFAULT => Error::new(ErrorKind::CtxInvalid),
                errno::EMFILE => Error::new(ErrorKind::SocketLimit),
                // The context associated with the handle is being terminated.
                errno::ETERM => Error::new(ErrorKind::CtxInvalid),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            // Set ZAP domain handling to strictly adhere the RFC.
            // This will eventually be enabled by default by ØMQ.
            setsockopt_bool(ptr, SocketOption::EnforceDomain, true)?;
            // We hardset the same domain name for each sockets because I don't
            // see any use cases for them.
            setsockopt_str(
                ptr,
                SocketOption::ZapDomain,
                Some("global"),
            )?;

            Ok(Self {
                ctx,
                ptr,
                mechanism: Mutex::default(),
                heartbeat: Mutex::default(),
            })
        }
    }

    pub(crate) fn handle(&self) -> SocketHandle {
        SocketHandle(self.ptr)
    }

    pub(crate) fn connect(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        connect(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn bind(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        bind(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn disconnect(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        disconnect(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn unbind(&self, endpoint: &Endpoint) -> Result<(), Error> {
        let c_string = CString::new(endpoint.to_zmq()).unwrap();
        unbind(self.as_mut_ptr(), c_string)
    }

    pub(crate) fn ctx(&self) -> CtxHandle {
        self.ctx
    }

    /// This is safe since the pointed socket is thread safe.
    pub(crate) fn as_mut_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub(crate) fn mechanism(&self) -> &Mutex<Mechanism> {
        &self.mechanism
    }

    pub(crate) fn heartbeat(&self) -> &Mutex<Option<Heartbeat>> {
        &self.heartbeat
    }

    pub(crate) fn last_endpoint(&self) -> Result<Option<Endpoint>, Error> {
        let maybe =
            getsockopt_string(self.as_mut_ptr(), SocketOption::LastEndpoint)?;

        Ok(maybe.map(|s| Endpoint::from_zmq(s.as_str())))
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

    pub(crate) fn set_heartbeat_ttl(
        &self,
        duration: Duration,
    ) -> Result<(), Error> {
        let ms = duration.as_millis();
        if ms > MAX_HB_TTL as u128 {
            return Err(Error::new(ErrorKind::InvalidInput(
                "duration ms cannot exceed 6553599",
            )));
        }
        setsockopt_duration(
            self.as_mut_ptr(),
            SocketOption::HeartbeatTtl,
            duration,
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

    pub(crate) fn recv_hwm(&self) -> Result<i32, Error> {
        getsockopt_scalar(self.as_mut_ptr(), SocketOption::RecvHighWaterMark)
    }

    pub(crate) fn set_recv_hwm(&self, hwm: i32) -> Result<(), Error> {
        if hwm <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput(
                "high water mark must be greater than zero",
            )));
        }
        setsockopt_scalar(
            self.as_mut_ptr(),
            SocketOption::RecvHighWaterMark,
            hwm,
        )
    }

    pub(crate) fn recv_timeout(&self) -> Result<Period, Error> {
        getsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::RecvTimeout,
            -1,
        )
        .map(Into::into)
    }

    pub(crate) fn set_recv_timeout(&self, period: Period) -> Result<(), Error> {
        setsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::RecvTimeout,
            period.into(),
            -1,
        )
    }

    pub(crate) fn send_hwm(&self) -> Result<i32, Error> {
        getsockopt_scalar(self.as_mut_ptr(), SocketOption::SendHighWaterMark)
    }

    pub(crate) fn set_send_hwm(&self, hwm: i32) -> Result<(), Error> {
        if hwm <= 0 {
            return Err(Error::new(ErrorKind::InvalidInput(
                "high water mark must be greater than zero",
            )));
        }
        setsockopt_scalar(
            self.as_mut_ptr(),
            SocketOption::SendHighWaterMark,
            hwm,
        )
    }

    pub(crate) fn send_timeout(&self) -> Result<Period, Error> {
        getsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::SendTimeout,
            -1,
        )
        .map(Into::into)
    }

    pub(crate) fn set_send_timeout(&self, period: Period) -> Result<(), Error> {
        setsockopt_option_duration(
            self.as_mut_ptr(),
            SocketOption::SendTimeout,
            period.into(),
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
        self.ptr == other.ptr
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
        let rc = unsafe { sys::zmq_close(self.ptr) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            error!("error while dropping socket: {}", msg_from_errno(errno));
        }
    }
}
