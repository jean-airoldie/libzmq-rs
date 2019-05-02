use crate::{core::*, error::*, Ctx};

use serde::{Deserialize, Serialize};

use std::{os::raw::c_void, sync::Arc};

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
    pub fn with_ctx(ctx: Ctx) -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Client, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        &self.inner.ctx
    }
}

impl GetRawSocket for Client {
    fn raw_socket(&self) -> *const c_void {
        self.inner.socket
    }

    // This is safe as long as it is only used by libzmq.
    fn mut_raw_socket(&self) -> *mut c_void {
        self.inner.socket as *mut _
    }
}

impl Socket for Client {}
impl SendMsg for Client {}
impl RecvMsg for Client {}

unsafe impl Send for Client {}
unsafe impl Sync for Client {}

/// A builder for a `Client`.
///
/// Especially helpfull in config files.
///
/// # Example
/// ```
/// use libzmq::socket::ClientConfig;
///
/// let client = ClientConfig::new().build();
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(flatten)]
    socket_config: SocketConfig,
    #[serde(flatten)]
    send_config: SendConfig,
    #[serde(flatten)]
    recv_config: RecvConfig,
}

impl ClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Client, Error> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Client, Error> {
        let client = Client::with_ctx(ctx)?;
        self.apply(&client)?;

        Ok(client)
    }

    pub fn apply(&self, client: &Client) -> Result<(), Error> {
        self.socket_config.apply(client)?;
        self.send_config.apply(client)?;
        self.recv_config.apply(client)?;

        Ok(())
    }
}

impl GetSocketConfig for ClientConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for ClientConfig {}

impl GetRecvConfig for ClientConfig {
    fn recv_config(&self) -> &RecvConfig {
        &self.recv_config
    }

    fn mut_recv_config(&mut self) -> &mut RecvConfig {
        &mut self.recv_config
    }
}

impl ConfigureRecv for ClientConfig {}

impl GetSendConfig for ClientConfig {
    fn send_config(&self) -> &SendConfig {
        &self.send_config
    }

    fn mut_send_config(&mut self) -> &mut SendConfig {
        &mut self.send_config
    }
}

impl ConfigureSend for ClientConfig {}

pub struct ClientBuilder {
    inner: ClientConfig,
}

impl GetSocketConfig for ClientBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        self.inner.mut_socket_config()
    }
}

impl BuildSocket for ClientBuilder {}

impl GetSendConfig for ClientBuilder {
    fn send_config(&self) -> &SendConfig {
        self.inner.send_config()
    }

    fn mut_send_config(&mut self) -> &mut SendConfig {
        self.inner.mut_send_config()
    }
}

impl BuildSend for ClientBuilder {}

impl GetRecvConfig for ClientBuilder {
    fn recv_config(&self) -> &RecvConfig {
        self.inner.recv_config()
    }

    fn mut_recv_config(&mut self) -> &mut RecvConfig {
        self.inner.mut_recv_config()
    }
}

impl BuildRecv for ClientBuilder {}
