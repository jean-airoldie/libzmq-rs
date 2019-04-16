use crate::{
    core::{sockopt::*, *},
    error::*,
    Ctx,
};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

/// A `Radio` socket is used by a publisher to distribute data to [`Dish`]
/// sockets.
///
/// Each message belong to a group specified with [`set_group`].
/// Messages are distributed to all members of a group.
///
/// # Mute State
/// When a `Radio` socket enters the mute state due to having reached the
/// high water mark for a subscriber, then any messages that would be sent to
/// the subscriber in question shall instead be dropped until the mute state ends.
///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *};
/// use std::{thread, time::Duration};
///
/// const ENDPOINT: &str = "inproc://test";
///
/// // We create our sockets.
/// let radio = Radio::new()?;
/// // We configure the radio so that it doesnt drop in mute state.
/// // However this means that a slow `Dish` would slow down
/// // the `Radio`. We use this is this example because `connect`
/// // takes a few milliseconds, enough for the `Radio` to drop a few messages.
/// radio.set_no_drop(true)?;
/// let first = Dish::new()?;
/// let second = Dish::new()?;
///
///
/// // We connect them.
/// radio.bind(ENDPOINT)?;
/// first.connect(ENDPOINT)?;
/// second.connect(ENDPOINT)?;
///
/// // Each dish will only receive messages from that group.
/// first.join("A")?;
/// second.join("B")?;
///
/// // Lets publish some messages to subscribers.
/// let mut msg: Msg = "first msg".into();
/// msg.set_group("A")?;
/// radio.send(msg)?;
/// let mut msg: Msg = "second msg".into();
/// msg.set_group("B")?;
/// radio.send(msg)?;
///
/// // Lets receive the publisher's messages.
/// let mut msg = first.recv_msg()?;
/// assert_eq!("first msg", msg.to_str().unwrap());
/// let err = first.recv_poll(&mut msg).unwrap_err();
/// // Only the message from the first group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
///
/// second.recv(&mut msg)?;
/// assert_eq!("second msg", msg.to_str().unwrap());
/// let err = first.recv_poll(&mut msg).unwrap_err();
/// // Only the message from the second group was received.
/// assert_eq!(ErrorKind::WouldBlock, err.kind());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// # Summary of Characteristics
/// | Characteristic            | Value          |
/// |:-------------------------:|:--------------:|
/// | Compatible peer sockets   | [`Dish`]       |
/// | Direction                 | Unidirectional |
/// | Send/receive pattern      | Send only      |
/// | Incoming routing strategy | N/A            |
/// | Outgoing routing strategy | Fan out        |
/// | Action in mute state      | Drop           |
///
/// [`Dish`]: struct.Dish.html
/// [`set_group`]: ../struct.Msg.html#method.set_group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Radio {
    inner: Arc<RawSocket>,
}

impl Radio {
    impl_socket_methods!(Radio);

    /// Returns `true` if the `no_drop` option is set.
    pub fn no_drop(&self) -> Result<bool, Error<()>> {
        getsockopt_bool(self.as_mut_raw_socket(), SocketOption::NoDrop)
    }

    /// Sets the socket's behaviour to block instead of drop messages when
    /// in the `mute state`.
    ///
    /// # Default value
    /// `false`
    ///
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`send_high_water_mark`]: #method.send_high_water_mark
    pub fn set_no_drop(&self, enabled: bool) -> Result<(), Error<()>> {
        setsockopt_bool(self.as_mut_raw_socket(), SocketOption::NoDrop, enabled)
    }
}

impl_as_raw_socket_trait!(Radio);
impl Socket for Radio {}

impl SendMsg for Radio {}

unsafe impl Send for Radio {}
unsafe impl Sync for Radio {}

/// A builder for a `Radio`.
///
/// Especially helpfull in config files.
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RadioConfig {
    inner: SocketConfig,
    no_drop: Option<bool>,
}

impl RadioConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Radio, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Radio, Error<()>> {
        let radio = Radio::with_ctx(ctx)?;
        self.apply(&radio)?;

        if let Some(enabled) = self.no_drop {
            radio.set_no_drop(enabled)?;
        }

        Ok(radio)
    }
}

impl_config_trait!(RadioConfig);
