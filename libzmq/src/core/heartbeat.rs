use super::{private, GetRawSocket, Period, RawSocket};
use crate::error::Error;
use Period::*;

use serde::{Deserialize, Serialize};

use std::{sync::MutexGuard, time::Duration};

fn set_heartbeat(
    raw_socket: &RawSocket,
    maybe: Option<Heartbeat>,
    mut mutex: MutexGuard<Option<Heartbeat>>,
) -> Result<(), Error> {
    if *mutex == maybe {
        return Ok(());
    }

    if let Some(heartbeat) = &maybe {
        raw_socket.set_heartbeat_interval(heartbeat.interval)?;
        if let Finite(timeout) = heartbeat.timeout {
            raw_socket.set_heartbeat_timeout(timeout)?;
        }
        if let Finite(ttl) = heartbeat.ttl {
            raw_socket.set_heartbeat_timeout(ttl)?;
        }
    } else {
        raw_socket.set_heartbeat_interval(Duration::from_millis(0))?;
        raw_socket.set_heartbeat_timeout(Duration::from_millis(0))?;
        raw_socket.set_heartbeat_ttl(Duration::from_millis(0))?;
    }

    *mutex = maybe;
    Ok(())
}

/// Socket heartbeating configuration.
///
/// # Example
/// ```
/// use libzmq::Heartbeat;
/// use std::time::Duration;
///
/// let duration = Duration::from_millis(300);
///
/// let hb = Heartbeat::new(duration)
///     .add_timeout(2 * duration);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Heartbeat {
    #[serde(with = "humantime_serde")]
    pub(crate) interval: Duration,
    pub(crate) timeout: Period,
    pub(crate) ttl: Period,
}

impl Heartbeat {
    /// Create a new `Heartbeat` from the given interval.
    ///
    /// This interval specifies the duration between each heartbeat.
    pub fn new<D>(interval: D) -> Self
    where
        D: Into<Duration>,
    {
        Self {
            interval: interval.into(),
            timeout: Infinite,
            ttl: Infinite,
        }
    }

    /// Returns the interval between each heartbeat.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Set a timeout for the `Heartbeat`.
    ///
    /// This timeout specifies how long to wait before timing out a connection
    /// with a peer for not receiving any traffic.
    pub fn add_timeout<D>(mut self, timeout: D) -> Self
    where
        D: Into<Duration>,
    {
        self.timeout = Finite(timeout.into());
        self
    }

    /// Returns the heartbeat timeout.
    pub fn timeout(&self) -> Period {
        self.timeout
    }

    /// Set a ttl for the `Heartbeat`
    ///
    /// This ttl is equivalent to a `heartbeat_timeout` for the remote
    /// side for this specific connection.
    pub fn add_ttl<D>(mut self, ttl: D) -> Self
    where
        D: Into<Duration>,
    {
        self.ttl = Finite(ttl.into());
        self
    }

    /// Returns the heartbeat ttl.
    pub fn ttl(&self) -> Period {
        self.ttl
    }
}

impl<'a> From<&'a Heartbeat> for Heartbeat {
    fn from(hb: &'a Heartbeat) -> Self {
        hb.to_owned()
    }
}

/// A trait that indicates that the socket supports configurable
/// heartbeating.
///
/// The actual heartbeating will be done by the engine in the background.
///
/// Only applies to connection based transports such as `TCP`.
///
/// # Contract
/// * timeout and interval duration in ms cannot exceed i32::MAX
/// * ttl duration in ms cannot exceed 6553599
///
/// # Default value
/// `None` (no heartbeating)
///
/// # Return Errors
/// * [`InvalidInput`]: (if contract is not respected)
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, Client, Heartbeat, auth::*};
/// use std::time::Duration;
///
/// let client = Client::new()?;
/// assert_eq!(client.heartbeat(), None);
///
/// let duration = Duration::from_millis(300);
/// let hb = Heartbeat::new(duration)
///     .add_timeout(2 * duration);
/// let expected = hb.clone();
///
/// client.set_heartbeat(Some(hb))?;
/// assert_eq!(client.heartbeat(), Some(expected));
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Mechanism`]: ../auth/enum.Mechanism.html
pub trait Heartbeating: GetRawSocket {
    /// Returns a the socket's heartbeating configuration.
    fn heartbeat(&self) -> Option<Heartbeat> {
        self.raw_socket().heartbeat().lock().unwrap().to_owned()
    }

    /// Sets the socket's heartbeating configuration.
    fn set_heartbeat(&self, maybe: Option<Heartbeat>) -> Result<(), Error> {
        let raw_socket = self.raw_socket();
        let mutex = raw_socket.heartbeat().lock().unwrap();

        set_heartbeat(raw_socket, maybe, mutex)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct HeartbeatingConfig {
    pub(crate) heartbeat: Option<Heartbeat>,
}

impl HeartbeatingConfig {
    pub(crate) fn apply<S: Heartbeating>(
        &self,
        socket: &S,
    ) -> Result<(), Error> {
        socket.set_heartbeat(self.heartbeat.clone())
    }
}

#[doc(hidden)]
pub trait GetHeartbeatingConfig: private::Sealed {
    fn heartbeat_config(&self) -> &HeartbeatingConfig;

    fn heartbeat_config_mut(&mut self) -> &mut HeartbeatingConfig;
}

impl GetHeartbeatingConfig for HeartbeatingConfig {
    fn heartbeat_config(&self) -> &HeartbeatingConfig {
        self
    }

    fn heartbeat_config_mut(&mut self) -> &mut HeartbeatingConfig {
        self
    }
}

/// A set of provided methods for a socket configuration.
pub trait ConfigureHeartbeating: GetHeartbeatingConfig {
    fn heartbeat(&self) -> Option<&Heartbeat> {
        self.heartbeat_config().heartbeat.as_ref()
    }

    fn set_heartbeat(&mut self, maybe: Option<Heartbeat>) {
        self.heartbeat_config_mut().heartbeat = maybe;
    }
}

impl ConfigureHeartbeating for HeartbeatingConfig {}

/// A set of provided methods for a socket builder.
pub trait BuildHeartbeating: GetHeartbeatingConfig + Sized {
    fn heartbeat<H>(&mut self, heartbeat: H) -> &mut Self
    where
        H: Into<Heartbeat>,
    {
        self.heartbeat_config_mut()
            .set_heartbeat(Some(heartbeat.into()));
        self
    }
}
