use crate::{core::*, error::*, Ctx, GroupOwned};
use libzmq_sys as sys;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{
    ffi::{c_void, CString},
    str,
    sync::{Arc, Mutex, MutexGuard},
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

    /// Returns a `MutexGuard` containing all the currently joined groups.
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
    pub fn joined(&self) -> MutexGuard<Vec<GroupOwned>> {
        self.groups.lock().unwrap()
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DishConfig {
    #[serde(flatten)]
    socket_config: SocketConfig,
    #[serde(flatten)]
    recv_config: RecvConfig,
    #[serde(flatten)]
    groups: Option<Vec<GroupOwned>>,
}

impl DishConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Dish, failure::Error> {
        self.build_with_ctx(Ctx::global())
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Dish, failure::Error>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let dish = Dish::with_ctx(ctx)?;
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

    pub fn apply(&self, dish: &Dish) -> Result<(), failure::Error> {
        self.socket_config.apply(dish)?;
        self.recv_config.apply(dish)?;

        if let Some(ref groups) = self.groups {
            for group in groups {
                dish.join(group)?;
            }
        }

        Ok(())
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

    pub fn build(&self) -> Result<Dish, failure::Error> {
        self.inner.build()
    }

    pub fn build_with_ctx<C>(&self, ctx: C) -> Result<Dish, failure::Error>
    where
        C: Into<Ctx>,
    {
        self.inner.build_with_ctx(ctx)
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
