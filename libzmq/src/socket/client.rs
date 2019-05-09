use crate::{core::*, error::*, Ctx, Endpoint};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{sync::Arc, time::Duration};

/// A `Client` socket is used for advanced request-reply messaging.
///
/// `Client` sockets are threadsafe and can be used from multiple threads at the
/// same time. Note that replies from a `Server` socket will go to the first
/// client thread that calls `recv`. If you need to get replies back to the
/// originating thread, use one `Client` socket per thread.
///
/// When a `Client` socket is connected to multiple sockets, outgoing
/// messages are distributed between connected peers on a round-robin basis.
/// Likewise, the `Client` socket receives messages fairly from each connected peer.
///
/// # Mute State
/// When `Client` socket enters the mute state due to having reached the high water
/// mark, or if there are no peers at all, then any `send operations
/// on the socket shall block unitl the mute state ends or at least one peer becomes
/// available for sending; messages are not discarded.
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Server`]             |
/// | Direction                 | Bidirectional          |
/// | Send/receive pattern      | Unrestricted           |
/// | Outgoing routing strategy | Round-robin            |
/// | Incoming routing strategy | Fair-queued            |
/// | Action in mute state      | Block                  |
///
/// [`Server`]: struct.Server.html
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    inner: Arc<RawSocket>,
}

impl Client {
    /// Create a `Client` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: ../ctx/struct.Ctx.html#method.global
    pub fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Client)?);

        Ok(Self { inner })
    }

    /// Create a `Client` socket from a specific context.
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
        let ctx = ctx.into();
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Client, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }
}

impl GetRawSocket for Client {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}

impl Socket for Client {}
impl SendMsg for Client {}
impl RecvMsg for Client {}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

/// A configuration for a `Client`.
///
/// Especially helpfull in config files.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ClientConfig {
    socket_config: SocketConfig,
    send_config: SendConfig,
    recv_config: RecvConfig,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Client, failure::Error> {
        self.build_with_ctx(Ctx::global())
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Client, failure::Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let client = Client::with_ctx(ctx)?;
        self.apply(&client)?;

        Ok(client)
    }

    pub fn apply(&self, client: &Client) -> Result<(), failure::Error> {
        self.socket_config.apply(client)?;
        self.send_config.apply(client)?;
        self.recv_config.apply(client)?;

        Ok(())
    }
}

// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
// Wish there was a better way.
#[derive(Serialize, Deserialize)]
struct FlatClientConfig {
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
    recv_high_water_mark: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    recv_timeout: Option<Duration>,
}

impl<'a> From<&'a ClientConfig> for FlatClientConfig {
    fn from(config: &'a ClientConfig) -> Self {
        let socket_config = &config.socket_config;
        let send_config = &config.send_config;
        let recv_config = &config.recv_config;
        Self {
            connect: socket_config.connect.to_owned(),
            bind: socket_config.bind.to_owned(),
            backlog: socket_config.backlog.to_owned(),
            connect_timeout: socket_config.connect_timeout.to_owned(),
            heartbeat_interval: socket_config.heartbeat_interval.to_owned(),
            heartbeat_timeout: socket_config.heartbeat_timeout.to_owned(),
            heartbeat_ttl: socket_config.heartbeat_ttl.to_owned(),
            linger: socket_config.linger.to_owned(),
            send_high_water_mark: send_config.send_high_water_mark.to_owned(),
            send_timeout: send_config.send_timeout.to_owned(),
            recv_high_water_mark: recv_config.recv_high_water_mark.to_owned(),
            recv_timeout: recv_config.recv_timeout.to_owned(),
        }
    }
}

impl From<FlatClientConfig> for ClientConfig {
    fn from(flat: FlatClientConfig) -> Self {
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
        let recv_config = RecvConfig {
            recv_high_water_mark: flat.recv_high_water_mark,
            recv_timeout: flat.recv_timeout,
        };
        Self {
            socket_config,
            send_config,
            recv_config,
        }
    }
}

impl Serialize for ClientConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let flattened: FlatClientConfig = self.into();
        flattened.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ClientConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let flat = FlatClientConfig::deserialize(deserializer)?;
        Ok(flat.into())
    }
}

impl GetSocketConfig for ClientConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for ClientConfig {}

impl GetRecvConfig for ClientConfig {
    fn recv_config(&self) -> &RecvConfig {
        &self.recv_config
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        &mut self.recv_config
    }
}

impl ConfigureRecv for ClientConfig {}

impl GetSendConfig for ClientConfig {
    fn send_config(&self) -> &SendConfig {
        &self.send_config
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        &mut self.send_config
    }
}

impl ConfigureSend for ClientConfig {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientBuilder {
    inner: ClientConfig,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Client, failure::Error> {
        self.inner.build()
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Client, failure::Error>
    where
        C: Into<Ctx>,
    {
        self.inner.build_with_ctx(ctx)
    }
}

impl GetSocketConfig for ClientBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self.inner.socket_config_mut()
    }
}

impl BuildSocket for ClientBuilder {}

impl GetSendConfig for ClientBuilder {
    fn send_config(&self) -> &SendConfig {
        self.inner.send_config()
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        self.inner.send_config_mut()
    }
}

impl BuildSend for ClientBuilder {}

impl GetRecvConfig for ClientBuilder {
    fn recv_config(&self) -> &RecvConfig {
        self.inner.recv_config()
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        self.inner.recv_config_mut()
    }
}

impl BuildRecv for ClientBuilder {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ser_de() {
        let config = ClientConfig::new();

        let ron = ron::ser::to_string(&config).unwrap();
        let de: ClientConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }
}
