use crate::{core::*, error::*, Ctx};

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
    impl_socket_methods!(Client);
}

impl_get_raw_socket_trait!(Client);
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

    pub fn build(&self) -> Result<Client, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Client, Error<()>> {
        let client = Client::with_ctx(ctx)?;
        self.apply(&client)?;

        Ok(client)
    }

    pub fn apply(&self, client: &Client) -> Result<(), Error<()>> {
        self.apply_socket_config(client)?;
        self.apply_send_config(client)?;
        self.apply_recv_config(client)?;

        Ok(())
    }
}

impl_get_socket_config_trait!(ClientConfig);
impl ConfigureSocket for ClientConfig {}

impl_get_send_config_trait!(ClientConfig);
impl ConfigureSend for ClientConfig {}

impl_get_recv_config_trait!(ClientConfig);
impl ConfigureRecv for ClientConfig {}
