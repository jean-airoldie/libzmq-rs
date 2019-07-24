use crate::{
    core::{raw::AsRawSocket, *},
    error::{msg_from_errno, Error, ErrorKind},
    msg::Msg,
};
use libzmq_sys as sys;
use sys::errno;

use std::{
    os::raw::{c_int, c_void},
    time::Duration,
};

fn recv(
    socket_ptr: *mut c_void,
    msg: &mut Msg,
    no_block: bool,
) -> Result<(), Error> {
    let rc = unsafe {
        sys::zmq_msg_recv(msg.as_mut_ptr(), socket_ptr, no_block as c_int)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = match errno {
            errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
            errno::ENOTSUP => panic!("recv not supported by socket type"),
            errno::EFSM => {
                panic!("operation cannot be completed in current socket state")
            }
            errno::ETERM => Error::new(ErrorKind::CtxInvalid),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EINTR => Error::new(ErrorKind::Interrupted),
            errno::EFAULT => panic!("invalid message"),
            _ => panic!(msg_from_errno(errno)),
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
    /// This operation might block until the socket receives a message or,
    /// if it is set, until `recv_timeout` expires.
    ///
    /// # Error
    /// The `Msg` is returned as the content of the `Error`.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`] (if `recv_timeout` expires)
    /// * [`CtxInvalid`]
    /// * [`Interrupted`]
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxInvalid`]: ../enum.ErrorKind.html#variant.CtxInvalid
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn recv(&self, msg: &mut Msg) -> Result<(), Error> {
        recv(self.raw_socket().as_mut_ptr(), msg, false)
    }

    /// Try to retrieve a message from the inbound socket queue without blocking.
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
    /// * [`CtxInvalid`]
    /// * [`Interrupted`]
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxInvalid`]: ../enum.ErrorKind.html#variant.CtxInvalid
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    fn try_recv(&self, msg: &mut Msg) -> Result<(), Error> {
        recv(self.raw_socket().as_mut_ptr(), msg, true)
    }

    /// A convenience function that allocates a [`Msg`] with the same properties
    /// as [`recv`].
    ///
    /// [`recv`]: #method.recv
    /// [`Msg`]: ../msg/struct.Msg.html
    fn recv_msg(&self) -> Result<Msg, Error> {
        let mut msg = Msg::new();
        self.recv(&mut msg)?;

        Ok(msg)
    }

    /// A convenience function that allocates a [`Msg`] with the same properties
    /// as [`try_recv`].
    ///
    /// [`try_recv`]: #method.recv
    /// [`Msg`]: ../msg/struct.Msg.html
    fn try_recv_msg(&self) -> Result<Msg, Error> {
        let mut msg = Msg::new();
        self.try_recv(&mut msg)?;

        Ok(msg)
    }

    /// The high water mark for incoming messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// incoming messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    ///
    /// # Default
    /// `1000`
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, *};
    ///
    /// let client = ClientBuilder::new().build()?;
    /// assert_eq!(client.recv_hwm()?, 1000);
    ///
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    fn recv_hwm(&self) -> Result<i32, Error> {
        self.raw_socket().recv_hwm()
    }

    /// Set the high water mark for inbound messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// outstanding messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    ///
    /// # Usage Contract
    /// * The high water mark cannot be zero.
    ///
    /// # Returned Error
    /// * [`InvalidInput`]
    ///
    /// # Default
    /// 1000
    ///
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    fn set_recv_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.raw_socket().set_recv_hwm(hwm)
    }

    /// The timeout for [`recv`] on the socket.
    ///
    /// If some timeout is specified, [`recv`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise it
    /// will until a message is received.
    fn recv_timeout(&self) -> Result<Period, Error> {
        self.raw_socket().recv_timeout()
    }

    /// Sets the timeout for [`recv`] on the socket.
    ///
    /// If some timeout is specified, [`recv`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise it
    /// will until a message is received.
    ///
    /// # Default
    /// `Infinite`
    fn set_recv_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.raw_socket().set_recv_timeout(period.into())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct RecvConfig {
    pub(crate) recv_hwm: HighWaterMark,
    pub(crate) recv_timeout: Period,
}

impl RecvConfig {
    pub(crate) fn apply<S: RecvMsg>(&self, socket: &S) -> Result<(), Error> {
        socket.set_recv_hwm(self.recv_hwm.into())?;
        socket.set_recv_timeout(self.recv_timeout)?;

        Ok(())
    }
}

#[doc(hidden)]
pub trait GetRecvConfig: private::Sealed {
    fn recv_config(&self) -> &RecvConfig;

    fn recv_config_mut(&mut self) -> &mut RecvConfig;
}

/// A set of provided methods for the configuration of a socket that implements `RecvMsg`.
pub trait ConfigureRecv: GetRecvConfig {
    fn recv_hwm(&self) -> i32 {
        self.recv_config().recv_hwm.into()
    }

    fn set_recv_hwm(&mut self, hwm: i32) {
        self.recv_config_mut().recv_hwm = HighWaterMark(hwm);
    }

    fn recv_timeout(&self) -> Period {
        self.recv_config().recv_timeout
    }

    fn set_recv_timeout(&mut self, period: Period) {
        self.recv_config_mut().recv_timeout = period;
    }
}

/// A set of provided methods for the builder of a socket that implements `RecvMsg`.
pub trait BuildRecv: GetRecvConfig {
    fn recv_hwm(&mut self, hwm: i32) -> &mut Self {
        self.recv_config_mut().recv_hwm = HighWaterMark(hwm);
        self
    }

    fn recv_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.recv_config_mut().recv_timeout = Finite(timeout);
        self
    }
}
