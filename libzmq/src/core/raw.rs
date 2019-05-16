use crate::{
    addr::Endpoint, auth::*, core::sockopt::*, error::*,
    monitor::init_socket_monitor, Ctx, InprocAddr,
};
use libzmq_sys as sys;
use sys::errno;

use log::error;
use serde::{Deserialize, Serialize};

use std::{
    ffi::CString,
    os::raw::{c_int, c_void},
    sync::Mutex,
};

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

#[derive(Debug)]
#[doc(hidden)]
pub struct RawSocket {
    socket_mut_ptr: *mut c_void,
    ctx: Ctx,
    connected: Mutex<Vec<Endpoint>>,
    bound: Mutex<Vec<Endpoint>>,
    mechanism: Mutex<Mechanism>,
    monitor_addr: InprocAddr,
}

impl RawSocket {
    pub(crate) fn new(sock_type: RawSocketType) -> Result<Self, Error> {
        let ctx = Ctx::global().clone();
        Self::with_ctx(sock_type, ctx)
    }

    pub(crate) fn with_ctx(
        sock_type: RawSocketType,
        ctx: Ctx,
    ) -> Result<Self, Error> {
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

            let monitor_addr = InprocAddr::new_unique();

            init_socket_monitor(socket_mut_ptr, &monitor_addr);

            Ok(Self {
                ctx,
                socket_mut_ptr,
                connected: Mutex::default(),
                bound: Mutex::default(),
                mechanism: Mutex::default(),
                monitor_addr,
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

    pub(crate) fn connected(&self) -> &Mutex<Vec<Endpoint>> {
        &self.connected
    }

    pub(crate) fn bound(&self) -> &Mutex<Vec<Endpoint>> {
        &self.bound
    }

    pub(crate) fn mechanism(&self) -> &Mutex<Mechanism> {
        &self.mechanism
    }

    pub(crate) fn monitor_addr(&self) -> &InprocAddr {
        &self.monitor_addr
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
        // This allows for the endpoints to be available for binding as soon as
        // the socket is dropped, as opposed to when ØMQ decides so.
        if let Ok(bound) = self.bound.lock() {
            for endpoint in &*bound {
                let _ = self.unbind(&endpoint);
            }
        }

        let rc = unsafe { sys::zmq_close(self.socket_mut_ptr) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            error!("error while dropping socket: {}", msg_from_errno(errno));
        }
    }
}
