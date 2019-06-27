use crate::{addr::Endpoint, auth::*, core::*, error::*, *};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

/// A `Server` socket is a socket used for advanced request-reply messaging.
///
/// A `Server` socket talks to a set of [`Client`] sockets. The [`Client`] must
/// first initiate the conversation, which generates a [`routing_id`] associated
/// with the connection. Each message received from a `Server` will have this
/// [`routing_id`]. To send messages back to the server, you must associate them
/// to a `RoutingId` either manually using [`set_routing_id`] or via the [`route`]
/// convenience method. If the [`routing_id`] is not specified, or
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
/// use libzmq::{prelude::*, *};
///
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let server = ServerBuilder::new()
///     .bind(addr)
///     .build()?;
///
/// let bound = server.last_endpoint()?;
///
/// let client = ClientBuilder::new()
///     .connect(bound)
///     .build()?;
///
/// // The client initiates the conversation so it is assigned a `routing_id`.
/// client.send("request")?;
/// let msg = server.recv_msg()?;
/// assert_eq!("request", msg.to_str()?);
/// let id = msg.routing_id().unwrap();
///
/// // Using this `routing_id`, we can now route as many replies as we
/// // want to the client.
/// server.route("reply 1", id)?;
/// server.route("reply 2", id)?;
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
/// [`route`]: #method.route
/// [`Client`]: struct.Client.html
/// [`routing_id`]: struct.Msg.html#method.routing_id
/// [`set_routing_id`]: struct.Msg.html#method.set_routing_id
/// [`HostUnreachable`]: enum.ErrorKind.html#variant.host-unreachable
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
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: ctx/struct.Ctx.html#method.global
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
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx<C>(ctx: C) -> Result<Server, Error>
    where
        C: Into<Ctx>,
    {
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Server, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }

    /// Push a message into the outgoing socket queue with the specified
    /// `RoutingId`.
    ///
    /// This is a convenience function that sets the `Msg`'s `RoutingId` then
    /// sends it.
    ///
    /// See [`send`] for more information.
    ///
    /// [`send`]: prelude/trait.SendMsg.html#method.send
    pub fn route<M>(&self, msg: M, id: RoutingId) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        let mut msg = msg.into();
        msg.set_routing_id(id);
        self.send(msg)
    }

    /// Try to push a message into the outgoing socket queue with the specified
    /// `RoutingId`.
    ///
    /// This is a convenience function that sets the `Msg`'s `RoutingId` then
    /// tries sends it.
    ///
    /// See [`try_send`] for more information.
    ///
    /// [`try_send`]: prelude/trait.SendMsg.html#method.try_send
    pub fn try_route<M>(&self, msg: M, id: RoutingId) -> Result<(), Error<Msg>>
    where
        M: Into<Msg>,
    {
        let mut msg = msg.into();
        msg.set_routing_id(id);
        self.try_send(msg)
    }
}

impl GetRawSocket for Server {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}

impl Heartbeating for Server {}
impl Socket for Server {}
impl SendMsg for Server {}
impl RecvMsg for Server {}

unsafe impl Send for Server {}
unsafe impl Sync for Server {}

/// A configuration for a `Server`.
///
/// Especially helpfull in config files.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "FlatServerConfig")]
#[serde(into = "FlatServerConfig")]
pub struct ServerConfig {
    socket_config: SocketConfig,
    send_config: SendConfig,
    recv_config: RecvConfig,
    heartbeat_config: HeartbeatingConfig,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Server, Error<usize>> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Server, Error<usize>>
    where
        C: Into<Ctx>,
    {
        let server = Server::with_ctx(ctx.into()).map_err(Error::cast)?;
        self.apply(&server)?;

        Ok(server)
    }

    pub fn apply(&self, server: &Server) -> Result<(), Error<usize>> {
        self.send_config.apply(server).map_err(Error::cast)?;
        self.recv_config.apply(server).map_err(Error::cast)?;
        self.heartbeat_config.apply(server).map_err(Error::cast)?;
        self.socket_config.apply(server)?;

        Ok(())
    }
}

// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
// Wish there was a better way.
#[derive(Clone, Serialize, Deserialize)]
struct FlatServerConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    heartbeat: Option<Heartbeat>,
    send_high_water_mark: HighWaterMark,
    send_timeout: Period,
    send_batch_size: BatchSize,
    recv_high_water_mark: HighWaterMark,
    recv_timeout: Period,
    recv_batch_size: BatchSize,
    mechanism: Option<Mechanism>,
}

impl From<ServerConfig> for FlatServerConfig {
    fn from(config: ServerConfig) -> Self {
        let socket_config = config.socket_config;
        let send_config = config.send_config;
        let recv_config = config.recv_config;
        let heartbeat_config = config.heartbeat_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            heartbeat: heartbeat_config.heartbeat,
            mechanism: socket_config.mechanism,
            send_high_water_mark: send_config.send_high_water_mark,
            send_timeout: send_config.send_timeout,
            send_batch_size: send_config.send_batch_size,
            recv_high_water_mark: recv_config.recv_high_water_mark,
            recv_timeout: recv_config.recv_timeout,
            recv_batch_size: recv_config.recv_batch_size,
        }
    }
}

impl From<FlatServerConfig> for ServerConfig {
    fn from(flat: FlatServerConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            mechanism: flat.mechanism,
        };
        let send_config = SendConfig {
            send_high_water_mark: flat.send_high_water_mark,
            send_timeout: flat.send_timeout,
            send_batch_size: flat.send_batch_size,
        };
        let recv_config = RecvConfig {
            recv_high_water_mark: flat.recv_high_water_mark,
            recv_timeout: flat.recv_timeout,
            recv_batch_size: flat.recv_batch_size,
        };
        let heartbeat_config = HeartbeatingConfig {
            heartbeat: flat.heartbeat,
        };
        Self {
            socket_config,
            send_config,
            recv_config,
            heartbeat_config,
        }
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

impl GetHeartbeatingConfig for ServerConfig {
    fn heartbeat_config(&self) -> &HeartbeatingConfig {
        &self.heartbeat_config
    }

    fn heartbeat_config_mut(&mut self) -> &mut HeartbeatingConfig {
        &mut self.heartbeat_config
    }
}

impl ConfigureHeartbeating for ServerConfig {}

/// A builder for a `Server`.
///
/// Allows for ergonomic one line socket configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ServerBuilder {
    inner: ServerConfig,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Server, Error<usize>> {
        self.inner.build()
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Server, Error<usize>>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
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

impl GetHeartbeatingConfig for ServerBuilder {
    fn heartbeat_config(&self) -> &HeartbeatingConfig {
        self.inner.heartbeat_config()
    }

    fn heartbeat_config_mut(&mut self) -> &mut HeartbeatingConfig {
        self.inner.heartbeat_config_mut()
    }
}

impl BuildHeartbeating for ServerBuilder {}

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
