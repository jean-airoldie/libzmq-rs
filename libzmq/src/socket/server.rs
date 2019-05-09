use crate::{core::*, error::*, Ctx, Endpoint};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{sync::Arc, time::Duration};

/// A `Server` socket is a socket used for advanced request-reply messaging.
///
/// `Server` sockets are threadsafe and do not accept the [`MORE`] flag.
///
/// A `Server` socket talks to a set of [`Server`] sockets. The [`Server`] must
/// first initiate the conversation, which generates a [`routing_id`] associated
/// with the connection. Each message received from a `Server` will have this
/// [`routing_id`]. To send messages back to the server, you must
/// [`set_routing_id`] on the messages. If the [`routing_id`] is not specified, or
/// does not refer to a connected server peer, the send call will fail with
/// [`HostUnreachable`].
///
/// # Mute State
/// When a `Server` socket enters the mute state due to having reached the high
/// water mark for all servers, or if there are no servers at
/// all, then any `send` operations on the socket shall block until the mute
/// state ends or at least one downstream node becomes available for sending;
/// messages are not discarded.
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Server`]             |
/// | Direction                 | Bidirectional          |
/// | Pattern                   | Unrestricted           |
/// | Incoming routing strategy | Fair-queued            |
/// | Outgoing routing strategy | See text               |
/// | Action in mute state      | Block                  |
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, socket::*, Msg, addr::InprocAddr};
/// use std::convert::TryInto;
///
/// let addr: InprocAddr = "test".try_into()?;
///
/// let client = ClientBuilder::new()
///     .connect(&addr)
///     .build()?;
///
/// let server = ServerBuilder::new()
///     .bind(addr)
///     .build()?;
///
/// // The client initiates the conversation so it is assigned a `routing_id`.
/// client.send("request")?;
/// let msg = server.recv_msg()?;
/// assert_eq!("request", msg.to_str()?);
/// let routing_id = msg.routing_id().unwrap();
///
/// // Using this `routing_id`, we can now route as many replies as we
/// // want to the client.
/// let mut msg: Msg = "reply 1".into();
/// msg.set_routing_id(routing_id);
/// server.send(msg)?;
/// let mut msg: Msg = "reply 2".into();
/// msg.set_routing_id(routing_id);
/// server.send(msg)?;
///
/// // The `routing_id` is discarted when the message is sent to the client.
/// let mut msg = client.recv_msg()?;
/// assert_eq!("reply 1", msg.to_str()?);
/// assert!(msg.routing_id().is_none());
/// client.recv(&mut msg)?;
/// assert_eq!("reply 2", msg.to_str()?);
/// assert!(msg.routing_id().is_none());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`MORE`]: constant.MORE.html
/// [`Server`]: struct.Server.html
/// [`routing_id`]: ../msg/struct.Msg.html#method.routing_id
/// [`set_routing_id`]: ../msg/struct.Msg.html#method.set_routing_id
/// [`HostUnreachable`]: ../enum.ErrorKind.html#variant.host-unreachable
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    inner: Arc<RawSocket>,
}

impl Server {
    /// Create a `Server` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: ../ctx/struct.Ctx.html#method.global
    pub fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Server)?);

        Ok(Self { inner })
    }

    /// Create a `Server` socket from a specific context.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx<C>(ctx: C) -> Result<Server, Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Server, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }
}

impl GetRawSocket for Server {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}

impl Socket for Server {}
impl SendMsg for Server {}
impl RecvMsg for Server {}

unsafe impl Send for Server {}
unsafe impl Sync for Server {}

/// A configuration for a `Server`.
///
/// Especially helpfull in config files.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ServerConfig {
    socket_config: SocketConfig,
    send_config: SendConfig,
    recv_config: RecvConfig,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Server, failure::Error> {
        self.build_with_ctx(Ctx::global())
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Server, failure::Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let server = Server::with_ctx(ctx)?;
        self.apply(&server)?;

        Ok(server)
    }

    pub fn apply(&self, server: &Server) -> Result<(), failure::Error> {
        self.socket_config.apply(server)?;
        self.send_config.apply(server)?;
        self.recv_config.apply(server)?;

        Ok(())
    }
}

// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
// Wish there was a better way.
#[derive(Serialize, Deserialize)]
struct FlatServerConfig {
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

impl<'a> From<&'a ServerConfig> for FlatServerConfig {
    fn from(config: &'a ServerConfig) -> Self {
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

impl From<FlatServerConfig> for ServerConfig {
    fn from(flat: FlatServerConfig) -> Self {
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

impl Serialize for ServerConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let flattened: FlatServerConfig = self.into();
        flattened.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let flat = FlatServerConfig::deserialize(deserializer)?;
        Ok(flat.into())
    }
}

impl GetSocketConfig for ServerConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for ServerConfig {}

impl GetRecvConfig for ServerConfig {
    fn recv_config(&self) -> &RecvConfig {
        &self.recv_config
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        &mut self.recv_config
    }
}

impl ConfigureRecv for ServerConfig {}

impl GetSendConfig for ServerConfig {
    fn send_config(&self) -> &SendConfig {
        &self.send_config
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        &mut self.send_config
    }
}

impl ConfigureSend for ServerConfig {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ServerBuilder {
    inner: ServerConfig,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Server, failure::Error> {
        self.inner.build()
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Server, failure::Error>
    where
        C: Into<Ctx>,
    {
        self.inner.build_with_ctx(ctx)
    }
}

impl GetSocketConfig for ServerBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self.inner.socket_config_mut()
    }
}

impl BuildSocket for ServerBuilder {}

impl GetSendConfig for ServerBuilder {
    fn send_config(&self) -> &SendConfig {
        self.inner.send_config()
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        self.inner.send_config_mut()
    }
}

impl BuildSend for ServerBuilder {}

impl GetRecvConfig for ServerBuilder {
    fn recv_config(&self) -> &RecvConfig {
        self.inner.recv_config()
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        self.inner.recv_config_mut()
    }
}

impl BuildRecv for ServerBuilder {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ser_de() {
        let config = ServerConfig::new();

        let ron = ron::ser::to_string(&config).unwrap();
        let de: ServerConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }
}
