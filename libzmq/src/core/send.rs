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
        let err = match errno {
            errno::EAGAIN => Error::with_content(ErrorKind::WouldBlock, msg),
            errno::ENOTSUP => panic!("send is not supported by socket type"),
            errno::EINVAL => {
                panic!("multipart messages are not supported by socket type")
            }
            errno::EFSM => {
                panic!("operation cannot be completed in current socket state")
            }
            errno::ETERM => Error::with_content(ErrorKind::InvalidCtx, msg),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EINTR => Error::with_content(ErrorKind::Interrupted, msg),
            errno::EFAULT => panic!("invalid message"),
            errno::EHOSTUNREACH => {
                Error::with_content(ErrorKind::HostUnreachable, msg)
            }
            _ => panic!(msg_from_errno(errno)),
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
    /// Push a message into the outgoing message queue.
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
    /// * [`InvalidCtx`]
    /// * [`Interrupted`]
    /// * [`HostUnreachable`] (only for [`Server`] socket)
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`HostUnreachable`]: ../enum.ErrorKind.html#variant.HostUnreachable
    /// [`Server`]: struct.Server.html
    fn send<M>(&self, msg: M) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        send(self.raw_socket().as_mut_ptr(), msg.into(), false)
    }

    /// Try to push a message into the outgoing message queue without blocking.
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
    /// * [`InvalidCtx`]
    /// * [`Interrupted`]
    /// * [`HostUnreachable`] (only for [`Server`] socket)
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
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
    /// assert_eq!(client.send_hwm()?, 1000);
    ///
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    fn send_hwm(&self) -> Result<i32, Error> {
        self.raw_socket().send_hwm()
    }

    /// Set the high water mark for outbound messages on the specified socket.
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
    /// * [`InvalidInput`](on contract violation)
    ///
    /// # Default value
    /// 1000
    ///
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    fn set_send_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.raw_socket().set_send_hwm(hwm)
    }

    /// Sets the timeout for [`send`] on the socket.
    ///
    /// If some timeout is specified, the [`send`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise,
    /// it will block until the message is sent.
    fn send_timeout(&self) -> Result<Period, Error> {
        self.raw_socket().send_timeout()
    }

    /// Sets the timeout for [`send`] on the socket.
    ///
    /// If some timeout is specified, the [`send`] will return
    /// [`WouldBlock`] after the duration is elapsed. Otherwise,
    /// it will block until the message is sent.
    ///
    /// # Default Value
    /// `Infinite`
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
    fn set_send_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.raw_socket().set_send_timeout(period.into())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SendConfig {
    pub(crate) send_hwm: HighWaterMark,
    pub(crate) send_timeout: Period,
}

impl SendConfig {
    pub(crate) fn apply<S: SendMsg>(&self, socket: &S) -> Result<(), Error> {
        socket.set_send_hwm(self.send_hwm.into())?;
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
    fn send_hwm(&self) -> i32 {
        self.send_config().send_hwm.into()
    }

    fn set_send_hwm(&mut self, hwm: i32) {
        self.send_config_mut().send_hwm = HighWaterMark(hwm);
    }

    fn send_timeout(&self) -> Period {
        self.send_config().send_timeout
    }

    fn set_send_timeout(&mut self, period: Period) {
        self.send_config_mut().send_timeout = period;
    }
}

/// A set of provided methods for the builder of a socket that implements `SendMsg`.
pub trait BuildSend: GetSendConfig {
    fn send_hwm(&mut self, hwm: i32) -> &mut Self {
        self.send_config_mut().send_hwm = HighWaterMark(hwm);
        self
    }

    fn send_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.send_config_mut().send_timeout = Finite(timeout);
        self
    }
}
