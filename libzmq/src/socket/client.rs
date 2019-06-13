use crate::{addr::Endpoint, auth::*, core::*, error::*, Ctx};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

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
/// mark, or if there are no peers at all, then any send operations on the
/// socket shall block until the mute state ends or at least one peer becomes
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
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *};
/// use std::convert::TryInto;
///
/// // Use a system assigned port.
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let server = ServerBuilder::new()
///     .bind(addr)
///     .build()?;
///
/// // Retrieve the addr that was assigned.
/// let bound = server.last_endpoint()?;
///
/// let client = ClientBuilder::new()
///     .connect(bound)
///     .build()?;
///
/// // Send a string request.
/// client.send("tell me something")?;
///
/// // Receive the client request.
/// let msg = server.recv_msg()?;
/// let id = msg.routing_id().unwrap();
///
/// // Reply to the client.
/// let mut reply: Msg = "it takes 224 bits to store a i32 in java".into();
/// server.route(reply, id)?;
///
/// // We can reply twice if we want.
/// let mut reply: Msg = "also don't talk to me".into();
/// server.route(reply, id)?;
///
/// // Retreive the first reply.
/// let mut msg = client.recv_msg()?;
/// // And the second.
/// client.recv(&mut msg)?;
/// #
/// #     Ok(())
/// # }
/// ```
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
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: struct.Ctx.html#method.global
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
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
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
// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "FlatClientConfig")]
#[serde(from = "FlatClientConfig")]
pub struct ClientConfig {
    socket_config: SocketConfig,
    send_config: SendConfig,
    recv_config: RecvConfig,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Client, Error<usize>> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Client, Error<usize>>
    where
        C: Into<Ctx>,
    {
        let client = Client::with_ctx(ctx).map_err(Error::cast)?;
        self.apply(&client)?;

        Ok(client)
    }

    pub fn apply(&self, client: &Client) -> Result<(), Error<usize>> {
        self.send_config.apply(client).map_err(Error::cast)?;
        self.recv_config.apply(client).map_err(Error::cast)?;
        self.socket_config.apply(client)?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FlatClientConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    heartbeat: Option<Heartbeat>,
    send_high_water_mark: Quantity,
    send_timeout: Period,
    recv_high_water_mark: Quantity,
    recv_timeout: Period,
    mechanism: Option<Mechanism>,
}

impl From<ClientConfig> for FlatClientConfig {
    fn from(config: ClientConfig) -> Self {
        let socket_config = config.socket_config;
        let send_config = config.send_config;
        let recv_config = config.recv_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            heartbeat: socket_config.heartbeat,
            mechanism: socket_config.mechanism,
            send_high_water_mark: send_config.send_high_water_mark,
            send_timeout: send_config.send_timeout,
            recv_high_water_mark: recv_config.recv_high_water_mark,
            recv_timeout: recv_config.recv_timeout,
        }
    }
}

impl From<FlatClientConfig> for ClientConfig {
    fn from(flat: FlatClientConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            heartbeat: flat.heartbeat,
            mechanism: flat.mechanism,
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

/// A builder for a `Client`.
///
/// Allows for ergonomic one line socket configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientBuilder {
    inner: ClientConfig,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Client, Error<usize>> {
        self.inner.build()
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Client, Error<usize>>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
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
    use crate::InprocAddr;
    use std::convert::TryInto;

    #[test]
    fn test_ser_de() {
        let addr: InprocAddr = "test".try_into().unwrap();

        let mut config = ClientConfig::new();
        config.set_connect(Some(&addr));

        let ron = ron::ser::to_string(&config).unwrap();
        let de: ClientConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }
}
