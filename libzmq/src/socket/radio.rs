use crate::{
    addr::Endpoint,
    core::{sockopt::*, *},
    error::*,
    Ctx,
};

use serde::{Deserialize, Serialize};

use std::{sync::Arc, time::Duration};

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
/// use libzmq::{prelude::*, socket::*, InprocAddr, Msg, Group, ErrorKind};
/// use std::convert::TryInto;
///
/// let addr: InprocAddr = "test".try_into()?;
///
/// let a: &Group = "A".try_into()?;
/// let b: &Group = "B".try_into()?;
///
/// // We configure the radio so that it doesnt drop in mute state.
/// // However this means that a slow `Dish` would slow down
/// // the `Radio`. We use this is this example because `connect`
/// // takes a few milliseconds, enough for the `Radio` to drop a few messages.
/// let radio = RadioBuilder::new()
///     .bind(&addr)
///     .no_drop()
///     .build()?;
///
/// let dish_a = DishBuilder::new()
///     .connect(&addr)
///     .join(a)
///     .build()?;
///
/// let dish_b = DishBuilder::new()
///     .connect(&addr)
///     .join(b)
///     .build()?;
///
/// // Lets publish some messages to subscribers.
/// let mut msg: Msg = "first msg".into();
/// msg.set_group(a);
/// radio.send(msg)?;
/// let mut msg: Msg = "second msg".into();
/// msg.set_group(b);
/// radio.send(msg)?;
///
/// // Lets receive the publisher's messages.
/// let mut msg = dish_a.recv_msg()?;
/// assert_eq!(msg.group().unwrap(), a);
/// assert_eq!(msg.to_str().unwrap(), "first msg");
/// let err = dish_a.try_recv(&mut msg).unwrap_err();
///
/// // Only the message from the first group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// dish_b.recv(&mut msg)?;
/// assert_eq!(msg.group().unwrap(), b);
/// assert_eq!(msg.to_str().unwrap(), "second msg");
/// let err = dish_b.try_recv(&mut msg).unwrap_err();
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
    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Radio, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> Result<bool, Error> {
        getsockopt_bool(self.raw_socket().as_mut_ptr(), SocketOption::NoDrop)
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
        setsockopt_bool(
            self.raw_socket().as_mut_ptr(),
            SocketOption::NoDrop,
            enabled,
        )
    }
}

impl GetRawSocket for Radio {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
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
#[serde(from = "FlatRadioConfig")]
#[serde(into = "FlatRadioConfig")]
pub struct RadioConfig {
    socket_config: SocketConfig,
    send_config: SendConfig,
    no_drop: Option<bool>,
}

impl RadioConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Radio, Error<usize>> {
        self.build_with_ctx(Ctx::global())
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Radio, Error<usize>>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let radio = Radio::with_ctx(ctx).map_err(Error::cast)?;
        self.apply(&radio)?;

        Ok(radio)
    }

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> bool {
        self.no_drop.unwrap_or_default()
    }

    /// Returns `true` if the `no_drop` option is set.
    pub fn set_no_drop(&mut self, cond: bool) {
        self.no_drop = Some(cond);
    }

    pub fn apply(&self, radio: &Radio) -> Result<(), Error<usize>> {
        self.socket_config.apply(radio)?;
        self.send_config.apply(radio).map_err(Error::cast)?;

        if let Some(enabled) = self.no_drop {
            radio.set_no_drop(enabled).map_err(Error::cast)?;
        }

        Ok(())
    }
}

// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
// Wish there was a better way.
#[derive(Serialize, Deserialize)]
struct FlatRadioConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    backlog: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    connect_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_interval: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_ttl: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    linger: Option<Duration>,
    send_high_water_mark: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    send_timeout: Option<Duration>,
    no_drop: Option<bool>,
}

impl From<RadioConfig> for FlatRadioConfig {
    fn from(config: RadioConfig) -> Self {
        let socket_config = config.socket_config;
        let send_config = config.send_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            backlog: socket_config.backlog,
            connect_timeout: socket_config.connect_timeout,
            heartbeat_interval: socket_config.heartbeat_interval,
            heartbeat_timeout: socket_config.heartbeat_timeout,
            heartbeat_ttl: socket_config.heartbeat_ttl,
            linger: socket_config.linger,
            send_high_water_mark: send_config.send_high_water_mark,
            send_timeout: send_config.send_timeout,
            no_drop: config.no_drop,
        }
    }
}

impl From<FlatRadioConfig> for RadioConfig {
    fn from(flat: FlatRadioConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            backlog: flat.backlog,
            connect_timeout: flat.connect_timeout,
            heartbeat_interval: flat.heartbeat_interval,
            heartbeat_timeout: flat.heartbeat_timeout,
            heartbeat_ttl: flat.heartbeat_ttl,
            linger: flat.linger,
        };
        let send_config = SendConfig {
            send_high_water_mark: flat.send_high_water_mark,
            send_timeout: flat.send_timeout,
        };
        Self {
            socket_config,
            send_config,
            no_drop: flat.no_drop,
        }
    }
}

impl GetSocketConfig for RadioConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for RadioConfig {}

impl GetSendConfig for RadioConfig {
    fn send_config(&self) -> &SendConfig {
        &self.send_config
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        &mut self.send_config
    }
}

impl ConfigureSend for RadioConfig {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct RadioBuilder {
    inner: RadioConfig,
}

impl RadioBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn no_drop(&mut self) -> &mut Self {
        self.inner.set_no_drop(true);
        self
    }

    pub fn build(&self) -> Result<Radio, Error<usize>> {
        self.inner.build()
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Radio, Error<usize>>
    where
        C: Into<Ctx>,
    {
        self.inner.build_with_ctx(ctx)
    }
}

impl GetSocketConfig for RadioBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self.inner.socket_config_mut()
    }
}

impl BuildSocket for RadioBuilder {}

impl GetSendConfig for RadioBuilder {
    fn send_config(&self) -> &SendConfig {
        self.inner.send_config()
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        self.inner.send_config_mut()
    }
}

impl BuildSend for RadioBuilder {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ser_de() {
        let config = RadioConfig::new();

        let ron = ron::ser::to_string(&config).unwrap();
        let de: RadioConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }
}
