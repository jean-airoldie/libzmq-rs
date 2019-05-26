use crate::{
    core::*,
    error::{msg_from_errno, Error, ErrorKind},
    msg::Msg,
};
use libzmq_sys as sys;
use sys::errno;

use std::{
    os::raw::{c_int, c_void},
    time::Duration,
};

fn send(
    socket_ptr: *mut c_void,
    mut msg: Msg,
    no_block: bool,
) -> Result<(), Error<Msg>> {
    let rc = unsafe {
        sys::zmq_msg_send(msg.as_mut_ptr(), socket_ptr, no_block as c_int)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EAGAIN => {
                    Error::with_content(ErrorKind::WouldBlock, msg)
                }
                errno::ENOTSUP => {
                    panic!("send is not supported by socket type")
                }
                errno::EINVAL => panic!(
                    "multipart messages are not supported by socket type"
                ),
                errno::EFSM => panic!(
                    "operation cannot be completed in current socket state"
                ),
                errno::ETERM => {
                    Error::with_content(ErrorKind::CtxTerminated, msg)
                }
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => {
                    Error::with_content(ErrorKind::Interrupted, msg)
                }
                errno::EFAULT => panic!("invalid message"),
                errno::EHOSTUNREACH => {
                    Error::with_content(ErrorKind::HostUnreachable, msg)
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
    /// This operation might block until the mute state end or,
    /// if it set, `send_timeout` expires.
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
    /// * [`WouldBlock`] (if `send_timeout` expires)
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
    fn send<M>(&self, msg: M) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        send(self.raw_socket().as_mut_ptr(), msg.into(), false)
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
    fn try_send<M>(&self, msg: M) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        send(self.raw_socket().as_mut_ptr(), msg.into(), true)
    }

    /// The high water mark for outbound messages on the specified socket.
    ///
    /// The high water mark is a hard limit on the maximum number of
    /// outstanding messages ØMQ shall queue in memory.
    ///
    /// If this limit has been reached the socket shall enter the `mute state`.
    fn send_high_water_mark(&self) -> Result<Option<i32>, Error> {
        self.raw_socket().send_high_water_mark()
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
        maybe: Option<i32>,
    ) -> Result<(), Error> {
        self.raw_socket().set_send_high_water_mark(maybe)
    }

    /// Sets the timeout for [`send`] on the socket.
    ///
    /// If some timeout is specified, the [`send`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise,
    /// it will block until the message is sent.
    fn send_timeout(&self) -> Result<Option<Duration>, Error> {
        self.raw_socket().send_timeout()
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
    fn set_send_timeout(&self, maybe: Option<Duration>) -> Result<(), Error> {
        self.raw_socket().set_send_timeout(maybe)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SendConfig {
    pub(crate) send_high_water_mark: Option<i32>,
    pub(crate) send_timeout: Option<Duration>,
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

    fn send_config_mut(&mut self) -> &mut SendConfig;
}

/// A set of provided methods for the configuration of socket that implements `SendMsg`.
pub trait ConfigureSend: GetSendConfig {
    fn send_high_water_mark(&self) -> Option<i32> {
        self.send_config().send_high_water_mark
    }

    fn set_send_high_water_mark(&mut self, maybe: Option<i32>) {
        self.send_config_mut().send_high_water_mark = maybe;
    }

    fn send_timeout(&self) -> Option<Duration> {
        self.send_config().send_timeout
    }

    fn set_send_timeout(&mut self, maybe: Option<Duration>) {
        self.send_config_mut().send_timeout = maybe;
    }
}

/// A set of provided methods for the builder of a socket that implements `SendMsg`.
pub trait BuildSend: GetSendConfig {
    fn send_high_water_mark(&mut self, hwm: i32) -> &mut Self {
        self.send_config_mut().send_high_water_mark = Some(hwm);
        self
    }

    fn send_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.send_config_mut().send_timeout = Some(timeout);
        self
    }
}
