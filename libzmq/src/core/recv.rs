use crate::{
    core::{private, raw::GetRawSocket, sockopt::*},
    error::{msg_from_errno, Error, ErrorKind},
    msg::Msg,
};
use libzmq_sys as sys;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{
    os::raw::{c_int, c_void},
    time::Duration,
};

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
pub trait RecvMsg: GetRawSocket {
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
        recv(self.mut_raw_socket(), msg, false)
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
        recv(self.mut_raw_socket(), msg, true)
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
        let mut_raw_socket = self.raw_socket() as *mut _;
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
                    self.mut_raw_socket(),
                    SocketOption::RecvHighWaterMark,
                    limit,
                )
            }
            None => setsockopt_scalar(
                self.mut_raw_socket(),
                SocketOption::RecvHighWaterMark,
                0,
            ),
        }
    }

    /// The timeout for [`recv`] on the socket.
    ///
    /// If some timeout is specified, [`recv`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise it
    /// will until a message is received.
    fn recv_timeout(&self) -> Result<Option<Duration>, Error<()>> {
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::RecvTimeout)
    }

    /// Sets the timeout for [`recv`] on the socket.
    ///
    /// If some timeout is specified, [`recv`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise it
    /// will until a message is received.
    fn set_recv_timeout(
        &self,
        maybe_duration: Option<Duration>,
    ) -> Result<(), Error<()>> {
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::RecvTimeout,
            maybe_duration,
            -1,
        )
    }
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[doc(hidden)]
pub struct RecvConfig {
    recv_high_water_mark: Option<i32>,
    recv_timeout: Option<Duration>,
}

#[doc(hidden)]
pub trait GetRecvConfig: private::Sealed {
    fn recv_config(&self) -> &RecvConfig;

    fn mut_recv_config(&mut self) -> &mut RecvConfig;
}

/// Allows for configuration of `recv` socket options.
pub trait ConfigureRecv: GetRecvConfig {
    fn recv_high_water_mark(&mut self, hwm: i32) -> &mut Self {
        let mut config = self.mut_recv_config();
        config.recv_high_water_mark = Some(hwm);
        self
    }

    fn recv_timeout(&mut self, timeout: Option<Duration>) -> &mut Self {
        let mut config = self.mut_recv_config();
        config.recv_timeout = timeout;
        self
    }

    #[doc(hidden)]
    fn apply_recv_config<S: RecvMsg>(
        &self,
        socket: &S,
    ) -> Result<(), Error<()>> {
        let config = self.recv_config();

        socket.set_recv_high_water_mark(config.recv_high_water_mark)?;
        socket.set_recv_timeout(config.recv_timeout)?;

        Ok(())
    }
}
