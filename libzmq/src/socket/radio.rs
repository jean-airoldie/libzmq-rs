use crate::{addr::Endpoint, auth::*, core::*, error::*, Ctx};

use serde::{Deserialize, Serialize};

use std::{sync::Arc, time::Duration};

/// A `Radio` socket is used by a publisher to distribute data to [`Dish`]
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
/// # Summary of Characteristics
/// | Characteristic            | Value          |
/// |:-------------------------:|:--------------:|
/// | Compatible peer sockets   | [`Dish`]       |
/// | Direction                 | Unidirectional |
/// | Send/receive pattern      | Send only      |
/// | Incoming routing strategy | N/A            |
/// | Outgoing routing strategy | Fan out        |
/// | Action in mute state      | Drop           |
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *};
/// use std::{convert::TryInto, thread, time::Duration};
///
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let radio = RadioBuilder::new()
///     .bind(addr)
///     .build()?;
///
/// let bound = radio.last_endpoint().unwrap();
/// let a: &Group = "A".try_into()?;
/// let b: &Group = "B".try_into()?;
///
/// let dish_a = DishBuilder::new()
///     .connect(&bound)
///     .join(a)
///     .build()?;
///
/// let dish_b = DishBuilder::new()
///     .connect(bound)
///     .join(b)
///     .build()?;
///
/// // Start the feed. It has no conceptual start nor end, thus we
/// // don't synchronize with the subscribers.
/// thread::spawn(move || {
///     let a: &Group = "A".try_into().unwrap();
///     let b: &Group = "B".try_into().unwrap();
///     let mut count = 0;
///     loop {
///         let mut msg = Msg::new();
///         // Alternate between the two groups.
///         let group = {
///             if count % 2 == 0 {
///                 a
///             } else {
///                 b
///             }
///         };
///
///         msg.set_group(group);
///         radio.send(msg).unwrap();
///
///         thread::sleep(Duration::from_millis(1));
///         count += 1;
///     }
/// });
///
/// // Each dish will only receive the messages from their respective groups.
/// let msg = dish_a.recv_msg()?;
/// assert_eq!(msg.group().unwrap(), a);
///
/// let msg = dish_b.recv_msg()?;
/// assert_eq!(msg.group().unwrap(), b);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Dish`]: struct.Dish.html
/// [`set_group`]: struct.Msg.html#method.set_group
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
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: struct.Ctx.html#method.global
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
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Radio, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> Result<bool, Error> {
        self.inner.no_drop()
    }

    /// Sets the socket's behaviour to block instead of drop messages when
    /// in the `mute state`.
    ///
    /// # Default value
    /// `false`
    ///
    /// [`WouldBlock`]: enum.ErrorKind.html#variant.WouldBlock
    /// [`send_high_water_mark`]: #method.send_high_water_mark
    pub fn set_no_drop(&self, enabled: bool) -> Result<(), Error> {
        self.inner.set_no_drop(enabled)
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
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Radio, Error<usize>>
    where
        C: Into<Ctx>,
    {
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
        if let Some(enabled) = self.no_drop {
            radio.set_no_drop(enabled).map_err(Error::cast)?;
        }
        self.send_config.apply(radio).map_err(Error::cast)?;
        self.socket_config.apply(radio)?;

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
    #[serde(with = "humantime_serde")]
    heartbeat_interval: Option<Duration>,
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    heartbeat_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    heartbeat_ttl: Option<Duration>,
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    linger: Option<Duration>,
    send_high_water_mark: Option<i32>,
    #[serde(default)]
    #[serde(with = "humantime_serde")]
    send_timeout: Option<Duration>,
    no_drop: Option<bool>,
    mechanism: Option<Mechanism>,
}

impl From<RadioConfig> for FlatRadioConfig {
    fn from(config: RadioConfig) -> Self {
        let socket_config = config.socket_config;
        let send_config = config.send_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            backlog: socket_config.backlog,
            heartbeat_interval: socket_config.heartbeat_interval,
            heartbeat_timeout: socket_config.heartbeat_timeout,
            heartbeat_ttl: socket_config.heartbeat_ttl,
            linger: socket_config.linger,
            send_high_water_mark: send_config.send_high_water_mark,
            send_timeout: send_config.send_timeout,
            no_drop: config.no_drop,
            mechanism: socket_config.mechanism,
        }
    }
}

impl From<FlatRadioConfig> for RadioConfig {
    fn from(flat: FlatRadioConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            backlog: flat.backlog,
            heartbeat_interval: flat.heartbeat_interval,
            heartbeat_timeout: flat.heartbeat_timeout,
            heartbeat_ttl: flat.heartbeat_ttl,
            linger: flat.linger,
            mechanism: flat.mechanism,
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

/// A builder for a `Radio`.
///
/// Allows for ergonomic one line socket configuration.
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

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Radio, Error<usize>>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
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
