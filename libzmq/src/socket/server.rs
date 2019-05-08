use crate::{core::*, error::*, Ctx};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

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
/// use libzmq::{prelude::*, socket::*, Msg, Endpoint};
/// use std::convert::TryInto;
///
/// let endpoint: Endpoint = "inproc://test".try_into().unwrap();
///
/// let client = ClientBuilder::new()
///     .connect(&endpoint)
///     .build()?;
///
/// let server = ServerBuilder::new()
///     .bind(endpoint)
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(flatten)]
    socket_config: SocketConfig,
    #[serde(flatten)]
    send_config: SendConfig,
    #[serde(flatten)]
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
