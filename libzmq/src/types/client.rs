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
/// `Client` sockets do not accept the `MORE` flag on sends. This limits them to
/// single part data.
///
/// # Mute State
/// When `Client` socket enters the mute state due to having reached the high water
/// mark, or if there are no peers at all, then any `send operations
/// on the socket shall block unitl the mute state ends or at least one peer becomes
/// available for sending; messages are not discarded.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, Client, Endpoint};
///
/// const ENDPOINT: &str = "inproc://test";
///
/// // Lets illustrate a request reply pattern using 2 client messaging
/// // each other.
/// let mut first = Client::new()?;
/// let mut second = Client::new()?;
///
/// first.bind(ENDPOINT)?;
/// second.connect(ENDPOINT)?;
///
/// // Lets do the whole request-reply thing.
/// first.send("request")?;
///
/// let mut msg = second.recv_msg()?;
/// assert_eq!("request", msg.to_str()?);
///
/// second.send("reply")?;
///
/// first.recv(&mut msg)?;
/// assert_eq!("reply", msg.to_str()?);
///
/// // We can send as many replies as we want. We don't need to follow
/// // a strict one request equals one reply pattern.
/// second.send("another reply")?;
///
/// first.recv(&mut msg)?;
/// assert_eq!("another reply", msg.to_str()?);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Server`], [`Client`] |
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

impl_as_raw_socket_trait!(Client);
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
/// use libzmq::types::ClientConfig;
///
/// let client = ClientConfig::new().build();
/// ```
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientConfig {
    inner: SocketConfig,
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
}

impl_config_trait!(ClientConfig);
