use crate::{addr::Endpoint, auth::*, core::*, error::*, Ctx};

use serde::{Deserialize, Serialize};

use std::{str, sync::Arc, time::Duration};

/// A `Gather` socket is used to receive pipelined messages.
///
/// Messages are fair-queued from among all connected [`Scatter`] sockets.
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Scatter`]            |
/// | Direction                 | Unidirectional         |
/// | Send/receive pattern      | Receive only           |
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
///
/// let addr_a = InprocAddr::new_unique();
/// let addr_b = InprocAddr::new_unique();
///
/// // Create our two load balancers.
/// let lb_a = ScatterBuilder::new()
///     .bind(&addr_a)
///     .build()?;
/// let lb_b = ScatterBuilder::new()
///     .bind(&addr_b)
///     .build()?;
///
/// // Connect the worker to both load balancers.
/// let worker = GatherBuilder::new()
///     .connect(&[addr_a, addr_b])
///     .recv_high_water_mark(1)
///     .build()?;
///
/// for _ in 0..100 {
///     lb_a.try_send("a")?;
/// }
/// for _ in 0..100 {
///     lb_b.try_send("b")?;
/// }
///
/// // The messages received should be fair-queues from
/// // our two load balancers.
/// let mut msg = Msg::new();
/// for i in 0..200 {
///     worker.try_recv(&mut msg)?;
///     if i % 2 == 0 {
///         assert_eq!(msg.to_str(), Ok("a"));
///     } else {
///         assert_eq!(msg.to_str(), Ok("b"));
///     }
/// }
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Scatter`]: struct.Scatter.html
#[derive(Debug, Clone)]
pub struct Gather {
    inner: Arc<RawSocket>,
}

impl Gather {
    /// Create a `Gather` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: struct.Ctx.html#method.global
    pub fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Gather)?);

        Ok(Self { inner })
    }

    /// Create a `Gather` socket from a specific context.
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
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Gather, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }
}

impl PartialEq for Gather {
    fn eq(&self, other: &Gather) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Gather {}

impl GetRawSocket for Gather {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}

impl Socket for Gather {}
impl RecvMsg for Gather {}

unsafe impl Send for Gather {}
unsafe impl Sync for Gather {}

/// A configuration for a `Gather`.
///
/// Especially helpfull in config files.
// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "FlatGatherConfig")]
#[serde(into = "FlatGatherConfig")]
pub struct GatherConfig {
    socket_config: SocketConfig,
    recv_config: RecvConfig,
}

impl GatherConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Gather, Error<usize>> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Gather, Error<usize>>
    where
        C: Into<Ctx>,
    {
        let gather = Gather::with_ctx(ctx).map_err(Error::cast)?;
        self.apply(&gather)?;

        Ok(gather)
    }

    pub fn apply(&self, gather: &Gather) -> Result<(), Error<usize>> {
        self.recv_config.apply(gather).map_err(Error::cast)?;
        self.socket_config.apply(gather)?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FlatGatherConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    heartbeat: Option<Heartbeat>,
    linger: Period,
    recv_high_water_mark: Quantity,
    recv_timeout: Period,
    mechanism: Option<Mechanism>,
}

impl From<GatherConfig> for FlatGatherConfig {
    fn from(config: GatherConfig) -> Self {
        let socket_config = config.socket_config;
        let recv_config = config.recv_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            heartbeat: socket_config.heartbeat,
            linger: socket_config.linger,
            mechanism: socket_config.mechanism,
            recv_high_water_mark: recv_config.recv_high_water_mark,
            recv_timeout: recv_config.recv_timeout,
        }
    }
}

impl From<FlatGatherConfig> for GatherConfig {
    fn from(flat: FlatGatherConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            heartbeat: flat.heartbeat,
            linger: flat.linger,
            mechanism: flat.mechanism,
        };
        let recv_config = RecvConfig {
            recv_high_water_mark: flat.recv_high_water_mark,
            recv_timeout: flat.recv_timeout,
        };
        Self {
            socket_config,
            recv_config,
        }
    }
}
impl GetSocketConfig for GatherConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for GatherConfig {}

impl GetRecvConfig for GatherConfig {
    fn recv_config(&self) -> &RecvConfig {
        &self.recv_config
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        &mut self.recv_config
    }
}

impl ConfigureRecv for GatherConfig {}

/// A builder for a `Gather`.
///
/// Allows for ergonomic one line socket configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GatherBuilder {
    inner: GatherConfig,
}

impl GatherBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Gather, Error<usize>> {
        self.inner.build()
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Gather, Error<usize>>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
    }
}

impl GetSocketConfig for GatherBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self.inner.socket_config_mut()
    }
}

impl BuildSocket for GatherBuilder {}

impl GetRecvConfig for GatherBuilder {
    fn recv_config(&self) -> &RecvConfig {
        self.inner.recv_config()
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        self.inner.recv_config_mut()
    }
}

impl BuildRecv for GatherBuilder {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;

    #[test]
    fn test_ser_de() {
        let config = GatherConfig::new();

        let ron = ron::ser::to_string(&config).unwrap();
        let de: GatherConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }

    #[test]
    fn test_gather() {
        let addr_a = InprocAddr::new_unique();
        let addr_b = InprocAddr::new_unique();

        // Create our two load balancers.
        let lb_a = ScatterBuilder::new().bind(&addr_a).build().unwrap();
        let lb_b = ScatterBuilder::new().bind(&addr_b).build().unwrap();

        // Connected the worker to both load balancers.
        let worker = GatherBuilder::new()
            .connect(&[addr_a, addr_b])
            .recv_high_water_mark(1)
            .build()
            .unwrap();

        for _ in 0..100 {
            lb_a.try_send("a").unwrap();
        }
        for _ in 0..100 {
            lb_b.try_send("b").unwrap();
        }

        // The messages received should be fair-queues amongst
        // our two load balancers.
        let mut msg = Msg::new();
        for i in 0..200 {
            worker.try_recv(&mut msg).unwrap();
            if i % 2 == 0 {
                assert_eq!(msg.to_str(), Ok("a"));
            } else {
                assert_eq!(msg.to_str(), Ok("b"));
            }
        }
    }
}
