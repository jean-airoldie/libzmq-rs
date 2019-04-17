use crate::{error::*, Ctx};
use libzmq_sys as sys;
use sys::errno;

use log::error;

use std::os::raw::{c_int, c_void};

#[doc(hidden)]
pub trait GetRawSocket: super::private::Sealed {
    fn raw_socket(&self) -> *const c_void;

    fn mut_raw_socket(&self) -> *mut c_void;
}

pub(crate) enum RawSocketType {
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
pub(crate) struct RawSocket {
    pub(crate) ctx: Ctx,
    pub(crate) socket: *mut c_void,
}

impl RawSocket {
    pub(crate) fn new(sock_type: RawSocketType) -> Result<Self, Error<()>> {
        let ctx = Ctx::global().clone();

        Self::with_ctx(sock_type, ctx)
    }

    pub(crate) fn with_ctx(
        sock_type: RawSocketType,
        ctx: Ctx,
    ) -> Result<Self, Error<()>> {
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
            error!("error while dropping socket: {}", msg_from_errno(errno));
        }
    }
}
