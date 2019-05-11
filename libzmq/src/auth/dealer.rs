use crate::{
    core::{GetRawSocket, RawSocket, RawSocketType},
    error::*,
    Ctx, Msg,
};

use libzmq_sys as sys;
use sys::errno;

use libc::c_int;

use std::{os::raw::c_void, sync::Arc};

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
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dealer {
    inner: Arc<RawSocket>,
}

impl Dealer {
    pub(crate) fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Dealer)?);

        Ok(Self { inner })
    }

    pub(crate) fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx = ctx.into();
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Dealer, ctx)?);

        Ok(Self { inner })
    }

    pub(crate) fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }

    pub(crate) fn send_multipart<I, M>(
        &mut self,
        iter: I,
        _more: bool,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = M>,
        M: Into<Msg>,
    {
        let mut last = None;
        let raw = self.raw_socket();

        for msg in iter.into_iter() {
            let msg: Msg = msg.into();
            if last == None {
                last = Some(msg);
            } else {
                send(raw.as_mut_ptr(), last.take().unwrap(), true)?;
                last = Some(msg);
            }
        }
        if let Some(msg) = last {
            send(raw.as_mut_ptr(), msg, false)?;
        }
        Ok(())
    }

    pub(crate) fn recv_msg_multipart(&mut self) -> Result<Vec<Msg>, Error> {
        let mut vec = Vec::new();
        let raw = self.raw_socket();

        loop {
            let mut msg = Msg::new();
            recv(raw.as_mut_ptr(), &mut msg)?;
            let has_more = msg.has_more();
            vec.push(msg);
            if !has_more {
                break;
            }
        }
        Ok(vec)
    }
}

impl GetRawSocket for Dealer {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}
