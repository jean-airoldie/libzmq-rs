use crate::{addr::Endpoint, auth::*, core::*, error::*, Ctx, GroupOwned};
use libzmq_sys as sys;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{
    ffi::{c_void, CString},
    str,
    sync::{Arc, Mutex},
    time::Duration,
};

fn join(socket_mut_ptr: *mut c_void, group: &GroupOwned) -> Result<(), Error> {
    let c_str = CString::new(group.as_str()).unwrap();
    let rc = unsafe { sys::zmq_join(socket_mut_ptr, c_str.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                    msg: "cannot join group twice",
                }),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EMTHREAD => panic!("no i/o thread available"),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

fn leave(socket_mut_ptr: *mut c_void, group: &GroupOwned) -> Result<(), Error> {
    let c_str = CString::new(group.as_str()).unwrap();
    let rc = unsafe { sys::zmq_leave(socket_mut_ptr, c_str.as_ptr()) };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                    msg: "cannot leave a group that wasn't joined",
                }),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EMTHREAD => panic!("no i/o thread available"),
                _ => panic!(msg_from_errno(errno)),
            }
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
/// #
/// # use failure::Error;
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, InprocAddr, socket::*, Msg, Group};
/// use std::convert::TryInto;
///
/// let addr: InprocAddr = "test".try_into()?;
/// let group: &Group = "some group".try_into()?;
///
/// // Setting `no_drop = true` is an anti pattern meant for illustration
/// // purposes.
/// let radio = RadioBuilder::new()
///     .bind(&addr)
///     .no_drop()
///     .build()?;
///
/// let dish = DishBuilder::new()
///     .connect(&addr)
///     .join(group)
///     .build()?;
///
/// let mut msg: Msg = "".into();
/// msg.set_group(group);
///
/// radio.send(msg)?;
/// let msg = dish.recv_msg()?;
/// assert!(msg.is_empty());
/// assert_eq!(msg.group().unwrap(), group);
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
    groups: Arc<Mutex<Vec<GroupOwned>>>,
}

impl Dish {
    /// Create a `Dish` socket from the [`global context`]
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    /// [`global context`]: ../ctx/struct.Ctx.html#method.global
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
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
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
    /// use std::convert::TryInto;
    ///
    /// let group: &Group = "some group".try_into()?;
    /// let dish = Dish::new()?;
    /// dish.join(group)?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    pub fn join<I, G>(&self, groups: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = G>,
        G: Into<GroupOwned>,
    {
        let mut count = 0;
        let mut guard = self.groups.lock().unwrap();

        for group in groups.into_iter() {
            let group = group.into();
            join(self.raw_socket().as_mut_ptr(), &group)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            guard.push(group);
            count += 1;
        }
        Ok(())
    }

    /// Returns a snapshot of the list of joined `Group`.
    ///
    /// The list might be modified by another thread after it is returned.
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Dish, Group};
    /// use std::convert::TryInto;
    ///
    /// let first: &Group = "first group".try_into()?;
    /// let second: &Group = "second group".try_into()?;
    ///
    /// let dish = Dish::new()?;
    /// assert!(dish.joined().is_empty());
    ///
    /// dish.join(vec![first, second])?;
    /// assert_eq!(dish.joined().len(), 2);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn joined(&self) -> Vec<GroupOwned> {
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
    /// use libzmq::{prelude::*, Dish, GroupOwned};
    /// use std::convert::TryInto;
    ///
    /// let group: GroupOwned = "some group".to_owned().try_into()?;
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
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    pub fn leave<I, G>(&self, groups: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = G>,
        G: Into<GroupOwned>,
    {
        let mut count = 0;
        let mut guard = self.groups.lock().unwrap();

        for group in groups.into_iter() {
            let group = group.into();
            leave(self.raw_socket().as_mut_ptr(), &group)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            let position = guard.iter().position(|g| g == &group).unwrap();
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
    groups: Option<Vec<GroupOwned>>,
}

impl DishConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Dish, failure::Error> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Dish, failure::Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let dish = Dish::with_ctx(ctx).map_err(Error::cast)?;
        self.apply(&dish)?;

        Ok(dish)
    }

    pub fn groups(&self) -> Option<&[GroupOwned]> {
        self.groups.as_ref().map(Vec::as_slice)
    }

    pub fn set_groups<I>(&mut self, maybe_groups: Option<I>)
    where
        I: IntoIterator<Item = GroupOwned>,
    {
        let groups = maybe_groups.map(|g| g.into_iter().collect());
        self.groups = groups;
    }

    pub fn apply(&self, dish: &Dish) -> Result<(), Error<usize>> {
        self.socket_config.apply(dish)?;
        self.recv_config.apply(dish).map_err(Error::cast)?;

        if let Some(ref groups) = self.groups {
            for group in groups {
                dish.join(group)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct FlatDishConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    backlog: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    connect_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_interval: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    heartbeat_ttl: Option<Duration>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    linger: Option<Duration>,
    recv_high_water_mark: Option<i32>,
    #[serde(default)]
    #[serde(with = "serde_humantime")]
    recv_timeout: Option<Duration>,
    groups: Option<Vec<GroupOwned>>,
    mechanism: Option<Mechanism>,
}

impl From<DishConfig> for FlatDishConfig {
    fn from(config: DishConfig) -> Self {
        let socket_config = config.socket_config;
        let recv_config = config.recv_config;
        Self {
            connect: socket_config.connect,
            bind: socket_config.bind,
            backlog: socket_config.backlog,
            connect_timeout: socket_config.connect_timeout,
            heartbeat_interval: socket_config.heartbeat_interval,
            heartbeat_timeout: socket_config.heartbeat_timeout,
            heartbeat_ttl: socket_config.heartbeat_ttl,
            linger: socket_config.linger,
            mechanism: socket_config.mechanism,
            recv_high_water_mark: recv_config.recv_high_water_mark,
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
            backlog: flat.backlog,
            connect_timeout: flat.connect_timeout,
            heartbeat_interval: flat.heartbeat_interval,
            heartbeat_timeout: flat.heartbeat_timeout,
            heartbeat_ttl: flat.heartbeat_ttl,
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

    pub fn with_ctx<C>(&self, ctx: C) -> Result<Dish, failure::Error>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
    }

    pub fn join<I, G>(&mut self, groups: I) -> &mut Self
    where
        I: IntoIterator<Item = G>,
        G: Into<GroupOwned>,
    {
        let groups: Vec<GroupOwned> = groups.into_iter().map(G::into).collect();
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
}
