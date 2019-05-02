use crate::{
    core::{sockopt::*, *},
    error::*,
    Ctx,
};

use serde::{Deserialize, Serialize};

use std::{os::raw::c_void, sync::Arc};

/// A `Radio` socket is used by a publisher to distribute data to [`Radio`]
/// sockets.
///
/// Each message belong to a group specified with [`set_group`].
/// Messages are distributed to all members of a group.
///
/// # Mute State
/// When a `Radio` socket enters the mute state due to having reached the
/// high water mark for a subscriber, then any messages that would be sent to
/// the subscriber in question shall instead be dropped until the mute state ends.
///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *};
/// use std::{thread, time::Duration};
///
/// const ENDPOINT: &str = "inproc://test";
///
/// // We create our sockets.
/// let radio = Radio::new()?;
/// // We configure the radio so that it doesnt drop in mute state.
/// // However this means that a slow `Dish` would slow down
/// // the `Radio`. We use this is this example because `connect`
/// // takes a few milliseconds, enough for the `Radio` to drop a few messages.
/// radio.set_no_drop(true)?;
/// let first = Dish::new()?;
/// let second = Dish::new()?;
///
///
/// // We connect them.
/// radio.bind(ENDPOINT)?;
/// first.connect(ENDPOINT)?;
/// second.connect(ENDPOINT)?;
///
/// // Each dish will only receive messages from that group.
/// first.join("A")?;
/// second.join("B")?;
///
/// // Lets publish some messages to subscribers.
/// let mut msg: Msg = "first msg".into();
/// msg.set_group("A")?;
/// radio.send(msg)?;
/// let mut msg: Msg = "second msg".into();
/// msg.set_group("B")?;
/// radio.send(msg)?;
///
/// // Lets receive the publisher's messages.
/// let mut msg = first.recv_msg()?;
/// assert_eq!("first msg", msg.to_str().unwrap());
/// let err = first.try_recv(&mut msg).unwrap_err();
/// // Only the message from the first group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// second.recv(&mut msg)?;
/// assert_eq!("second msg", msg.to_str().unwrap());
/// let err = first.try_recv(&mut msg).unwrap_err();
/// // Only the message from the second group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value          |
/// |:-------------------------:|:--------------:|
/// | Compatible peer sockets   | [`Radio`]       |
/// | Direction                 | Unidirectional |
/// | Send/receive pattern      | Send only      |
/// | Incoming routing strategy | N/A            |
/// | Outgoing routing strategy | Fan out        |
/// | Action in mute state      | Drop           |
///
/// [`Radio`]: struct.Radio.html
/// [`set_group`]: ../struct.Msg.html#method.set_group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Radio {
    inner: Arc<RawSocket>,
}

impl Radio {
    /// Create a `Radio` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: ../ctx/struct.Ctx.html#method.global
    pub fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Radio)?);

        Ok(Self { inner })
    }

    /// Create a `Radio` socket from a specific context.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx(ctx: Ctx) -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Radio, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        &self.inner.ctx
    }

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> Result<bool, Error> {
        getsockopt_bool(self.mut_raw_socket(), SocketOption::NoDrop)
    }

    /// Sets the socket's behaviour to block instead of drop messages when
    /// in the `mute state`.
    ///
    /// # Default value
    /// `false`
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`send_high_water_mark`]: #method.send_high_water_mark
    pub fn set_no_drop(&self, enabled: bool) -> Result<(), Error> {
        setsockopt_bool(self.mut_raw_socket(), SocketOption::NoDrop, enabled)
    }
}

impl GetRawSocket for Radio {
    fn raw_socket(&self) -> *const c_void {
        self.inner.socket
    }

    // This is safe as long as it is only used by libzmq.
    fn mut_raw_socket(&self) -> *mut c_void {
        self.inner.socket as *mut _
    }
}

impl Socket for Radio {}
impl SendMsg for Radio {}

unsafe impl Send for Radio {}
unsafe impl Sync for Radio {}

/// A configuration for a `Radio`.
///
/// Especially helpfull in config files.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RadioConfig {
    #[serde(flatten)]
    socket_config: SocketConfig,
    #[serde(flatten)]
    send_config: SendConfig,
    #[serde(flatten)]
    recv_config: RecvConfig,
    #[serde(flatten)]
    no_drop: Option<bool>,
}

impl RadioConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Radio, Error> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Radio, Error> {
        let radio = Radio::with_ctx(ctx)?;
        self.apply(&radio)?;

        Ok(radio)
    }

    pub fn apply(&self, radio: &Radio) -> Result<(), Error> {
        self.socket_config.apply(radio)?;
        self.send_config.apply(radio)?;

        if let Some(enabled) = self.no_drop {
            radio.set_no_drop(enabled)?;
        }

        Ok(())
    }
}

impl GetSocketConfig for RadioConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for RadioConfig {}

impl GetSendConfig for RadioConfig {
    fn send_config(&self) -> &SendConfig {
        &self.send_config
    }

    fn mut_send_config(&mut self) -> &mut SendConfig {
        &mut self.send_config
    }
}

impl ConfigureSend for RadioConfig {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RadioBuilder {
    inner: RadioConfig,
}

impl RadioBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl GetSocketConfig for RadioBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        self.inner.mut_socket_config()
    }
}

impl BuildSocket for RadioBuilder {}

impl GetSendConfig for RadioBuilder {
    fn send_config(&self) -> &SendConfig {
        self.inner.send_config()
    }

    fn mut_send_config(&mut self) -> &mut SendConfig {
        self.inner.mut_send_config()
    }
}

impl BuildSend for RadioBuilder {}
