use crate::{
    addr::Endpoint, auth::*, core::*, error::*, Ctx, Group, GroupSlice,
};
use libzmq_sys as sys;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{
    ffi::c_void,
    str,
    sync::{Arc, Mutex},
};

fn join(socket_mut_ptr: *mut c_void, group: &GroupSlice) -> Result<(), Error> {
    let rc =
        unsafe { sys::zmq_join(socket_mut_ptr, group.as_c_str().as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = match errno {
            errno::EINVAL => {
                Error::new(ErrorKind::InvalidInput("cannot join group twice"))
            }
            errno::ETERM => Error::new(ErrorKind::CtxTerminated),
            errno::EINTR => Error::new(ErrorKind::Interrupted),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EMTHREAD => panic!("no i/o thread available"),
            _ => panic!(msg_from_errno(errno)),
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn leave(socket_mut_ptr: *mut c_void, group: &GroupSlice) -> Result<(), Error> {
    let rc =
        unsafe { sys::zmq_leave(socket_mut_ptr, group.as_c_str().as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = match errno {
            errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                "cannot leave a group that wasn't joined",
            )),
            errno::ETERM => Error::new(ErrorKind::CtxTerminated),
            errno::EINTR => Error::new(ErrorKind::Interrupted),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EMTHREAD => panic!("no i/o thread available"),
            _ => panic!(msg_from_errno(errno)),
        };

        Err(err)
    } else {
        Ok(())
    }
}

/// A `Dish` socket is used by a subscriber to subscribe to groups distributed
/// by a [`Radio`].
///
/// Initially a ZMQ_DISH socket is not subscribed to any groups, use [`join`]
/// to join a group.
///
/// # Summary of Characteristics
/// | Characteristic            | Value          |
/// |:-------------------------:|:--------------:|
/// | Compatible peer sockets   | [`Radio`]      |
/// | Direction                 | Unidirectional |
/// | Send/receive pattern      | Receive only   |
/// | Incoming routing strategy | Fair-queued    |
/// | Outgoing routing strategy | N/A            |
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *};
/// use std::{thread, time::Duration};
///
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let radio = RadioBuilder::new()
///     .bind(addr)
///     .build()?;
///
/// let bound = radio.last_endpoint().unwrap();
/// let a: Group = "group a".try_into()?;
///
/// let dish = DishBuilder::new()
///     .connect(bound)
///     .join(&a)
///     .build()?;
///
/// // Start the feed. It has no conceptual start nor end, thus we
/// // don't synchronize with the subscribers.
/// thread::spawn(move || {
///     let a: Group = "group a".try_into().unwrap();
///     let b: Group = "group b".try_into().unwrap();
///     let mut count = 0;
///     loop {
///         let mut msg = Msg::new();
///         // Alternate between the two groups.
///         let group = if count % 2 == 0 {
///             &a
///         } else {
///             &b
///         };
///
///         radio.transmit(msg, group).unwrap();
///
///         thread::sleep(Duration::from_millis(1));
///         count += 1;
///     }
/// });
///
/// // The dish exclusively receives messages from the groups it joined.
/// let msg = dish.recv_msg()?;
/// assert_eq!(msg.group().unwrap(), &a);
///
/// let msg = dish.recv_msg()?;
/// assert_eq!(msg.group().unwrap(), &a);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Radio`]: struct.Radio.html
/// [`join`]: #method.join
#[derive(Debug, Clone)]
pub struct Dish {
    inner: Arc<RawSocket>,
    groups: Arc<Mutex<Vec<Group>>>,
}

impl Dish {
    /// Create a `Dish` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: struct.Ctx.html#method.global
    pub fn new() -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::new(RawSocketType::Dish)?);

        Ok(Self {
            inner,
            groups: Arc::default(),
        })
    }

    /// Create a `Dish` socket from a specific context.
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
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Dish, ctx)?);

        Ok(Self {
            inner,
            groups: Arc::default(),
        })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        self.inner.ctx()
    }
    /// Joins the specified group(s).
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// groups that were joined before the failure.
    ///
    /// # Usage Contract
    /// * Each group can be joined at most once.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`InvalidInput`] (if group was already joined)
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Dish, Group};
    ///
    /// let group: Group = "some group".try_into()?;
    /// let dish = Dish::new()?;
    /// dish.join(group)?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: enum.ErrorKind.html#variant.InvalidInput
    pub fn join<I, G>(&self, groups: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = G>,
        G: Into<Group>,
    {
        let mut count = 0;
        let mut guard = self.groups.lock().unwrap();

        for group in groups.into_iter().map(G::into) {
            join(self.raw_socket().as_mut_ptr(), &group)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            guard.push(group);
            count += 1;
        }
        Ok(())
    }

    /// Returns a snapshot of the list of joined groups.
    ///
    /// The list might be modified by another thread after it is returned.
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Dish, Group};
    ///
    /// let first: Group = "first group".try_into()?;
    /// let second: Group = "second group".try_into()?;
    ///
    /// let dish = Dish::new()?;
    /// assert!(dish.joined().is_empty());
    ///
    /// dish.join(&[first, second])?;
    /// assert_eq!(dish.joined().len(), 2);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn joined(&self) -> Vec<Group> {
        self.groups.lock().unwrap().to_owned()
    }

    /// Leave the specified group(s).
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// groups that were leaved before the failure.
    ///
    /// # Usage Contract
    /// * The group must be already joined.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`InvalidInput`] (if group not already joined)
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Dish, Group};
    ///
    /// let group: Group = "some group".to_owned().try_into()?;
    ///
    /// let dish = Dish::new()?;
    /// assert!(dish.joined().is_empty());
    ///
    /// dish.join(&group)?;
    /// assert_eq!(dish.joined().len(), 1);
    ///
    /// dish.leave(&group)?;
    /// assert!(dish.joined().is_empty());
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    /// [`CtxTerminated`]: enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: enum.ErrorKind.html#variant.InvalidInput
    pub fn leave<I, G>(&self, groups: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = G>,
        G: AsRef<GroupSlice>,
    {
        let mut count = 0;
        let mut guard = self.groups.lock().unwrap();

        for group in groups.into_iter() {
            let group = group.as_ref();
            leave(self.raw_socket().as_mut_ptr(), group)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            let position = guard.iter().position(|g| g == group).unwrap();
            guard.remove(position);
            count += 1;
        }
        Ok(())
    }
}

impl PartialEq for Dish {
    fn eq(&self, other: &Dish) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Dish {}

impl GetRawSocket for Dish {
    fn raw_socket(&self) -> &RawSocket {
        &self.inner
    }
}

impl Socket for Dish {}
impl RecvMsg for Dish {}

unsafe impl Send for Dish {}
unsafe impl Sync for Dish {}

/// A configuration for a `Dish`.
///
/// Especially helpfull in config files.
// We can't derive and use #[serde(flatten)] because of this issue:
// https://github.com/serde-rs/serde/issues/1346
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "FlatDishConfig")]
#[serde(into = "FlatDishConfig")]
pub struct DishConfig {
    socket_config: SocketConfig,
    recv_config: RecvConfig,
    groups: Option<Vec<Group>>,
}

impl DishConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Dish, Error<usize>> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Dish, Error<usize>>
    where
        C: Into<Ctx>,
    {
        let dish = Dish::with_ctx(ctx).map_err(Error::cast)?;
        self.apply(&dish)?;

        Ok(dish)
    }

    pub fn groups(&self) -> Option<&[Group]> {
        self.groups.as_ref().map(Vec::as_slice)
    }

    pub fn set_groups<I>(&mut self, maybe_groups: Option<I>)
    where
        I: IntoIterator<Item = Group>,
    {
        let groups = maybe_groups.map(|g| g.into_iter().collect());
        self.groups = groups;
    }

    pub fn apply(&self, dish: &Dish) -> Result<(), Error<usize>> {
        if let Some(ref groups) = self.groups {
            dish.join(groups)?;
        }
        self.recv_config.apply(dish).map_err(Error::cast)?;
        self.socket_config.apply(dish)?;

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FlatDishConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    recv_hwm: HighWaterMark,
    recv_timeout: Period,
    groups: Option<Vec<Group>>,
    mechanism: Option<Mechanism>,
}

impl From<DishConfig> for FlatDishConfig {
    fn from(config: DishConfig) -> Self {
        let socket_config = config.socket_config;
        let recv_config = config.recv_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            mechanism: socket_config.mechanism,
            recv_hwm: recv_config.recv_hwm,
            recv_timeout: recv_config.recv_timeout,
            groups: config.groups,
        }
    }
}

impl From<FlatDishConfig> for DishConfig {
    fn from(flat: FlatDishConfig) -> Self {
        let socket_config = SocketConfig {
            connect: flat.connect,
            bind: flat.bind,
            mechanism: flat.mechanism,
        };
        let recv_config = RecvConfig {
            recv_hwm: flat.recv_hwm,
            recv_timeout: flat.recv_timeout,
        };
        Self {
            socket_config,
            recv_config,
            groups: flat.groups,
        }
    }
}
impl GetSocketConfig for DishConfig {
    fn socket_config(&self) -> &SocketConfig {
        &self.socket_config
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for DishConfig {}

impl GetRecvConfig for DishConfig {
    fn recv_config(&self) -> &RecvConfig {
        &self.recv_config
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        &mut self.recv_config
    }
}

impl ConfigureRecv for DishConfig {}

/// A builder for a `Dish`.
///
/// Allows for ergonomic one line socket configuration.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DishBuilder {
    inner: DishConfig,
}

impl DishBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Dish, Error<usize>> {
        self.inner.build()
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Dish, Error<usize>>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
    }

    pub fn join<I, G>(&mut self, groups: I) -> &mut Self
    where
        I: IntoIterator<Item = G>,
        G: Into<Group>,
    {
        let groups: Vec<Group> = groups.into_iter().map(G::into).collect();
        self.inner.set_groups(Some(groups));
        self
    }
}

impl GetSocketConfig for DishBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self.inner.socket_config_mut()
    }
}

impl BuildSocket for DishBuilder {}

impl GetRecvConfig for DishBuilder {
    fn recv_config(&self) -> &RecvConfig {
        self.inner.recv_config()
    }

    fn recv_config_mut(&mut self) -> &mut RecvConfig {
        self.inner.recv_config_mut()
    }
}

impl BuildRecv for DishBuilder {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ser_de() {
        let config = DishConfig::new();

        let ron = ron::ser::to_string(&config).unwrap();
        let de: DishConfig = ron::de::from_str(&ron).unwrap();
        assert_eq!(config, de);
    }

    #[test]
    fn test_dish() {
        use crate::{prelude::*, TcpAddr, *};
        use std::{thread, time::Duration};

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let radio = RadioBuilder::new().bind(addr).build().unwrap();

        let bound = radio.last_endpoint().unwrap();
        let a: Group = "group a".try_into().unwrap();

        let dish = DishBuilder::new().connect(bound).join(&a).build().unwrap();

        // Start the feed. It has no conceptual start nor end, thus we
        // don't synchronize with the subscribers.
        thread::spawn(move || {
            let a: Group = "group a".try_into().unwrap();
            let b: Group = "group b".try_into().unwrap();
            let mut count = 0;
            loop {
                let mut msg = Msg::new();
                // Alternate between the two groups.
                let group = if count % 2 == 0 { &a } else { &b };

                msg.set_group(group);
                radio.send(msg).unwrap();

                std::thread::sleep(Duration::from_millis(1));
                count += 1;
            }
        });

        // The dish exclusively receives messages from the groups it joined.
        let msg = dish.recv_msg().unwrap();
        assert_eq!(msg.group().unwrap(), &a);

        let msg = dish.recv_msg().unwrap();
        assert_eq!(msg.group().unwrap(), &a);
    }
}
