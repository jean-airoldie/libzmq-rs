use crate::{
    addr::Endpoint,
    core::{GetRawSocket, RawSocket, RawSocketType},
    error::*,
    Ctx, Msg,
};
use libzmq_sys as sys;
use sys::errno;

use libc::c_int;

use std::os::raw::c_void;

fn send(
    mut_sock_ptr: *mut c_void,
    mut msg: Msg,
    more: bool,
) -> Result<(), Error> {
    let flags = {
        if more {
            sys::ZMQ_SNDMORE
        } else {
            0
        }
    };
    let rc = unsafe {
        sys::zmq_msg_send(msg.as_mut_ptr(), mut_sock_ptr, flags as c_int)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn recv(mut_sock_ptr: *mut c_void, msg: &mut Msg) -> Result<(), Error> {
    let rc = unsafe { sys::zmq_msg_recv(msg.as_mut_ptr(), mut_sock_ptr, 0) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

pub(crate) enum OldSocketType {
    Router,
    Dealer,
    Pair,
    Sub,
}

impl From<OldSocketType> for RawSocketType {
    fn from(socket: OldSocketType) -> Self {
        match socket {
            OldSocketType::Router => RawSocketType::Router,
            OldSocketType::Dealer => RawSocketType::Dealer,
            OldSocketType::Pair => RawSocketType::Pair,
            OldSocketType::Sub => RawSocketType::Sub,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OldSocket {
    inner: RawSocket,
}

impl OldSocket {
    pub(crate) fn with_ctx<C>(
        socket: OldSocketType,
        ctx: C,
    ) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx = ctx.into();
        let inner = RawSocket::with_ctx(socket.into(), ctx)?;

        Ok(Self { inner })
    }

    pub(crate) fn bind<E>(&mut self, endpoint: E) -> Result<(), Error>
    where
        E: Into<Endpoint>,
    {
        let endpoint = endpoint.into();
        self.inner.bind(&endpoint)
    }

    pub(crate) fn send<M>(&mut self, msg: M, more: bool) -> Result<(), Error>
    where
        M: Into<Msg>,
    {
        let msg = msg.into();
        send(self.inner.as_mut_ptr(), msg, more)
    }

    pub(crate) fn send_multipart<I, M>(&mut self, iter: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = M>,
        M: Into<Msg>,
    {
        let mut last = None;

        for msg in iter.into_iter() {
            let msg: Msg = msg.into();
            if last == None {
                last = Some(msg);
            } else {
                self.send(last.take().unwrap(), true)?;
                last = Some(msg);
            }
        }
        if let Some(msg) = last {
            self.send(msg, false)?;
        }
        Ok(())
    }

    pub(crate) fn recv_msg_multipart(&mut self) -> Result<Vec<Msg>, Error> {
        let mut vec = Vec::new();
        loop {
            let mut msg = Msg::new();
            recv(self.inner.as_mut_ptr(), &mut msg)?;
            let has_more = msg.has_more();
            vec.push(msg);
            if !has_more {
                break;
            }
        }
        Ok(vec)
    }
}

unsafe impl Send for OldSocket {}

impl GetRawSocket for OldSocket {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}
