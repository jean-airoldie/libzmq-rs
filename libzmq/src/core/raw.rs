use crate::{
    addr::Endpoint,
    auth::{Creds, Mechanism},
    error::*,
    Ctx,
};
use libzmq_sys as sys;
use sys::errno;

use log::error;
use serde::{Deserialize, Serialize};

use std::{
    os::raw::{c_int, c_void},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Mutex,
    },
};

#[doc(hidden)]
pub trait GetRawSocket: super::private::Sealed {
    fn raw_socket(&self) -> &RawSocket;
}

pub(crate) enum RawSocketType {
    Client,
    Server,
    Radio,
    Dish,
    Dealer,
}

impl From<RawSocketType> for c_int {
    fn from(r: RawSocketType) -> c_int {
        match r {
            RawSocketType::Client => sys::ZMQ_CLIENT as c_int,
            RawSocketType::Server => sys::ZMQ_SERVER as c_int,
            RawSocketType::Radio => sys::ZMQ_RADIO as c_int,
            RawSocketType::Dish => sys::ZMQ_DISH as c_int,
            RawSocketType::Dealer => sys::ZMQ_DEALER as c_int,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthRole {
    Client = 0,
    Server,
}

impl From<bool> for AuthRole {
    fn from(b: bool) -> Self {
        match b {
            b if b == (AuthRole::Client as i32 != 0) => AuthRole::Client,
            b if b == (AuthRole::Server as i32 != 0) => AuthRole::Server,
            _ => unreachable!(),
        }
    }
}

impl From<AuthRole> for bool {
    fn from(r: AuthRole) -> Self {
        match r {
            AuthRole::Client => AuthRole::Client as i32 != 0,
            AuthRole::Server => AuthRole::Server as i32 != 0,
        }
    }
}

impl Default for AuthRole {
    fn default() -> Self {
        AuthRole::Client
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct RawSocket {
    socket_mut_ptr: *mut c_void,
    ctx: Ctx,
    connected: Mutex<Vec<Endpoint>>,
    bound: Mutex<Vec<Endpoint>>,
    creds: Mutex<Creds>,
    mechanism: AtomicU8,
    auth_role: AtomicBool,
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
            Ok(Self {
                ctx,
                socket_mut_ptr,
                connected: Mutex::default(),
                bound: Mutex::default(),
                mechanism: AtomicU8::default(),
                creds: Mutex::default(),
                auth_role: AtomicBool::default(),
            })
        }
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

    pub(crate) fn creds(&self) -> Creds {
        self.creds.lock().unwrap().to_owned()
    }

    pub(crate) fn set_creds(&self, _creds: Creds) -> Result<(), Error> {
        unimplemented!()
    }

    pub(crate) fn mechanism(&self) -> Mechanism {
        self.mechanism.load(Ordering::Relaxed).into()
    }

    pub(crate) fn set_mechanism(
        &self,
        mechanism: Mechanism,
    ) -> Result<(), Error> {
        self.mechanism.store(mechanism.into(), Ordering::Relaxed);
        unimplemented!()
    }

    pub(crate) fn auth_role(&self) -> AuthRole {
        self.auth_role.load(Ordering::Relaxed).into()
    }

    pub(crate) fn set_auth_role(&self, role: AuthRole) -> Result<(), Error> {
        self.auth_role.store(role.into(), Ordering::Relaxed);
        unimplemented!()
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
