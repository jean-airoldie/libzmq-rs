use crate::{addr::Endpoint, auth::*, core::*, error::*, Ctx, CtxHandle};

use serde::{Deserialize, Serialize};

use std::{str, sync::Arc};

/// A `Scatter` socket is used to pipeline messages to workers.
///
/// Messages are round-robined to all connected [`Gather`] sockets.
///
/// # Summary of Characteristics
/// | Characteristic            | Value                  |
/// |:-------------------------:|:----------------------:|
/// | Compatible peer sockets   | [`Gather`]             |
/// | Direction                 | Unidirectional         |
/// | Send/receive pattern      | Send only              |
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
/// use std::time::Duration;
///
/// let addr = InprocAddr::new_unique();
///
/// // Our load balancing producer.
/// let lb = ScatterBuilder::new()
///     .bind(&addr)
///     .build()?;
///
/// let worker_a = GatherBuilder::new()
///     .connect(&addr)
///     .recv_hwm(1)
///     .recv_timeout(Duration::from_millis(100))
///     .build()?;
///
/// let worker_b = GatherBuilder::new()
///     .connect(&addr)
///     .recv_hwm(1)
///     .recv_timeout(Duration::from_millis(100))
///     .build()?;
///
/// // Send messages to workers in a round-robin fashion.
/// lb.send("")?;
/// lb.send("")?;
///
/// assert!(worker_a.recv_msg()?.is_empty());
/// assert!(worker_b.recv_msg()?.is_empty());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Gather`]: struct.Gather.html
#[derive(Debug, Clone)]
pub struct Scatter {
    inner: Arc<RawSocket>,
}

impl Scatter {
    /// Create a `Scatter` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxInvalid`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxInvalid`]: enum.ErrorKind.html#variant.CtxInvalid
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: struct.Ctx.html#method.global
    pub fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Scatter)?);

        Ok(Self { inner })
    }

    /// Create a `Scatter` socket associated with a specific context
    /// from a `CtxHandle`.
    ///
    /// # Returned Error Variants
    /// * [`CtxInvalid`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxInvalid`]: enum.ErrorKind.html#variant.CtxInvalid
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx(handle: CtxHandle) -> Result<Self, Error> {
        let inner =
            Arc::new(RawSocket::with_ctx(RawSocketType::Scatter, handle)?);

        Ok(Self { inner })
    }

    /// Returns a copyable, thread-safe handle to the socket used for polling.
    pub fn handle(&self) -> SocketHandle {
        self.inner.handle()
    }

    /// Returns a handle to the context of the socket.
    pub fn ctx(&self) -> CtxHandle {
        self.inner.ctx()
    }
}

impl PartialEq for Scatter {
    fn eq(&self, other: &Scatter) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Scatter {}

impl AsRawSocket for Scatter {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}

impl Heartbeating for Scatter {}
impl Socket for Scatter {}
impl SendMsg for Scatter {}

unsafe impl Send for Scatter {}
unsafe impl Sync for Scatter {}

/// A configuration for a `Scatter`.
///
/// Especially helpfull in config files.
// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "FlatScatterConfig")]
#[serde(into = "FlatScatterConfig")]
pub struct ScatterConfig {
    socket_config: SocketConfig,
    send_config: SendConfig,
    heartbeat_config: HeartbeatingConfig,
}

impl ScatterConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Scatter, Error<usize>> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx(&self, handle: CtxHandle) -> Result<Scatter, Error<usize>> {
        let scatter = Scatter::with_ctx(handle).map_err(Error::cast)?;
        self.apply(&scatter)?;

        Ok(scatter)
    }

    pub fn apply(&self, scatter: &Scatter) -> Result<(), Error<usize>> {
        self.send_config.apply(scatter).map_err(Error::cast)?;
        self.socket_config.apply(scatter)?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FlatScatterConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    heartbeat: Option<Heartbeat>,
    send_hwm: HighWaterMark,
    send_timeout: Period,
    mechanism: Option<Mechanism>,
}

impl From<ScatterConfig> for FlatScatterConfig {
    fn from(config: ScatterConfig) -> Self {
        let socket_config = config.socket_config;
        let send_config = config.send_config;
        let heartbeat_config = config.heartbeat_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            heartbeat: heartbeat_config.heartbeat,
            mechanism: socket_config.mechanism,
            send_hwm: send_config.send_hwm,
            send_timeout: send_config.send_timeout,
        }
    }
}

impl From<FlatScatterConfig> for ScatterConfig {
    fn from(flat: FlatScatterConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            mechanism: flat.mechanism,
        };
        let send_config = SendConfig {
            send_hwm: flat.send_hwm,
            send_timeout: flat.send_timeout,
        };
        let heartbeat_config = HeartbeatingConfig {
            heartbeat: flat.heartbeat,
        };
        Self {
            socket_config,
            send_config,
            heartbeat_config,
        }
    }
}
impl GetSocketConfig for ScatterConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for ScatterConfig {}

impl GetSendConfig for ScatterConfig {
    fn send_config(&self) -> &SendConfig {
        &self.send_config
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        &mut self.send_config
    }
}

impl ConfigureSend for ScatterConfig {}

impl GetHeartbeatingConfig for ScatterConfig {
    fn heartbeat_config(&self) -> &HeartbeatingConfig {
        &self.heartbeat_config
    }

    fn heartbeat_config_mut(&mut self) -> &mut HeartbeatingConfig {
        &mut self.heartbeat_config
    }
}

impl ConfigureHeartbeating for ScatterConfig {}

/// A builder for a `Scatter`.
///
/// Allows for ergonomic one line socket configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScatterBuilder {
    inner: ScatterConfig,
}

impl ScatterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Scatter, Error<usize>> {
        self.inner.build()
    }

    pub fn with_ctx(&self, handle: CtxHandle) -> Result<Scatter, Error<usize>> {
        self.inner.with_ctx(handle)
    }
}

impl GetSocketConfig for ScatterBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self.inner.socket_config_mut()
    }
}

impl BuildSocket for ScatterBuilder {}

impl GetSendConfig for ScatterBuilder {
    fn send_config(&self) -> &SendConfig {
        self.inner.send_config()
    }

    fn send_config_mut(&mut self) -> &mut SendConfig {
        self.inner.send_config_mut()
    }
}

impl BuildSend for ScatterBuilder {}

impl GetHeartbeatingConfig for ScatterBuilder {
    fn heartbeat_config(&self) -> &HeartbeatingConfig {
        self.inner.heartbeat_config()
    }

    fn heartbeat_config_mut(&mut self) -> &mut HeartbeatingConfig {
        self.inner.heartbeat_config_mut()
    }
}

impl BuildHeartbeating for ScatterBuilder {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    use std::time::Duration;

    #[test]
    fn test_ser_de() {
        let config = ScatterConfig::new();

        let ron = ron::ser::to_string(&config).unwrap();
        let de: ScatterConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }

    #[test]
    fn test_scatter() {
        let addr = InprocAddr::new_unique();

        // Our load balancing producer.
        let lb = ScatterBuilder::new().bind(&addr).build().unwrap();

        let worker_a = GatherBuilder::new()
            .connect(&addr)
            .recv_hwm(1)
            .recv_timeout(Duration::from_millis(300))
            .build()
            .unwrap();

        let worker_b = GatherBuilder::new()
            .connect(&addr)
            .recv_hwm(1)
            .recv_timeout(Duration::from_millis(300))
            .build()
            .unwrap();

        // Send messages to workers in a round-robin fashion.
        lb.send("").unwrap();
        lb.send("").unwrap();

        assert!(worker_a.recv_msg().unwrap().is_empty());
        assert!(worker_b.recv_msg().unwrap().is_empty());
    }
}
