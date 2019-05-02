use crate::{
    core::*,
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

fn send(
    mut_raw_socket: *mut c_void,
    mut msg: Msg,
    no_block: bool,
) -> Result<(), Error> {
    let mut_msg_ptr = msg.as_mut_ptr();
    let rc = unsafe {
        sys::zmq_msg_send(mut_msg_ptr, mut_raw_socket, no_block as c_int)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EAGAIN => Error::with_msg(ErrorKind::WouldBlock, msg),
                errno::ENOTSUP => {
                    panic!("send is not supported by socket type")
                }
                errno::EINVAL => panic!(
                    "multipart messages are not supported by socket type"
                ),
                errno::EFSM => panic!(
                    "operation cannot be completed in current socket state"
                ),
                errno::ETERM => Error::with_msg(ErrorKind::CtxTerminated, msg),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => Error::with_msg(ErrorKind::Interrupted, msg),
                errno::EFAULT => panic!("invalid message"),
                errno::EHOSTUNREACH => {
                    Error::with_msg(ErrorKind::HostUnreachable, msg)
                }
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

/// Send messages in a thread-safe fashion.
///
/// Does not support multipart messages.
pub trait SendMsg: GetRawSocket {
    /// Push a message into the outgoing socket queue.
    ///
    /// This operation might block if the socket is in mute state.
    ///
    /// If the message is a `Msg`, `Vec<u8>`, `[u8]`, or a `String`, it is not copied.
    ///
    /// # Success
    /// The message was queued and now belongs to ØMQ
    ///
    /// # Error
    /// In case of an error, the message is not queued and
    /// the ownership is returned.
    ///
    /// ## Possible Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`HostUnreachable`] (only for [`Server`] socket)
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`HostUnreachable`]: ../enum.ErrorKind.html#variant.HostUnreachable
    /// [`Server`]: struct.Server.html
    fn send<M>(&self, sendable: M) -> Result<(), Error>
    where
        M: Into<Msg>,
    {
        let msg: Msg = sendable.into();
        send(self.mut_raw_socket(), msg, false)
    }

    /// Try to push a message into the outgoing socket queue without blocking.
    ///
    /// If the action would block, it returns a [`WouldBlock`] error, otherwise
    /// the message is pushed into the outgoing queue.
    ///
    /// If the message is a `Msg`, `Vec<u8>`, `[u8]`, or a `String`, it is not copied.
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
    fn try_send<M>(&self, sendable: M) -> Result<(), Error>
    where
        M: Into<Msg>,
    {
        let msg: Msg = sendable.into();
        send(self.mut_raw_socket(), msg, true)
    }

    /// The high water mark for outbound messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// outstanding messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    fn send_high_water_mark(&self) -> Result<Option<i32>, Error> {
        let mut_raw_socket = self.raw_socket() as *mut _;
        let limit =
            getsockopt_scalar(mut_raw_socket, SocketOption::SendHighWaterMark)?;

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
        &self,
        high_water_mark: Option<i32>,
    ) -> Result<(), Error> {
        match high_water_mark {
            Some(limit) => {
                assert!(limit != 0, "high water mark cannot be zero");
                setsockopt_scalar(
                    self.mut_raw_socket(),
                    SocketOption::SendHighWaterMark,
                    limit,
                )
            }
            None => setsockopt_scalar(
                self.mut_raw_socket(),
                SocketOption::SendHighWaterMark,
                0,
            ),
        }
    }

    /// Sets the timeout for [`send`] on the socket.
    ///
    /// If some timeout is specified, the [`send`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise,
    /// it will block until the message is sent.
    fn send_timeout(&self) -> Result<Option<Duration>, Error> {
        let mut_raw_socket = self.raw_socket() as *mut _;
        getsockopt_duration(mut_raw_socket, SocketOption::SendTimeout, -1)
    }

    /// Sets the timeout for [`send`] on the socket.
    ///
    /// If some timeout is specified, the [`send`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise,
    /// it will block until the message is sent.
    ///
    /// # Default Value
    /// `None`
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Client, ErrorKind};
    /// use std::time::Duration;
    ///
    /// let client = Client::new()?;
    /// client.set_send_timeout(Some(Duration::from_millis(1)))?;
    ///
    /// // The client is in mute state so the following would block forever
    /// // if a timeout wasn't specified. Instead, it will block for 1ms.
    /// let err = client.send("msg").unwrap_err();
    /// assert_eq!(ErrorKind::WouldBlock, err.kind());
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    fn set_send_timeout(&self, timeout: Option<Duration>) -> Result<(), Error> {
        setsockopt_duration(
            self.mut_raw_socket(),
            SocketOption::SendTimeout,
            timeout,
            -1,
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[doc(hidden)]
pub struct SendConfig {
    send_high_water_mark: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    send_timeout: Option<Duration>,
}

impl SendConfig {
    pub(crate) fn apply<S: SendMsg>(&self, socket: &S) -> Result<(), Error> {
        socket.set_send_high_water_mark(self.send_high_water_mark)?;
        socket.set_send_timeout(self.send_timeout)?;

        Ok(())
    }
}

#[doc(hidden)]
pub trait GetSendConfig: private::Sealed {
    fn send_config(&self) -> &SendConfig;

    fn mut_send_config(&mut self) -> &mut SendConfig;
}

pub trait ConfigureSend: GetSendConfig {
    fn send_high_water_mark(&self) -> Option<i32> {
        self.send_config().send_high_water_mark
    }

    fn send_timeout(&self) -> Option<Duration> {
        self.send_config().send_timeout
    }
}

pub trait BuildSend: GetSendConfig + Sized {
    fn send_high_water_mark(&mut self, hwm: i32) -> &mut Self {
        let mut config = self.mut_send_config();
        config.send_high_water_mark = Some(hwm);
        self
    }

    fn send_timeout(&mut self, timeout: Option<Duration>) -> &mut Self {
        let mut config = self.mut_send_config();
        config.send_timeout = timeout;
        self
    }
}
