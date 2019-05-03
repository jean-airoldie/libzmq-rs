use crate::{core::*, error::*, Group, GroupOwned, Ctx};
use libzmq_sys as sys;
use sys::errno;

use serde::{Deserialize, Serialize};

use std::{
    convert::{TryFrom, TryInto},
    ffi::CString,
    os::raw::c_void,
    str,
    sync::Arc,
};

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    inner: Arc<RawSocket>,
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

        Ok(Self { inner })
    }

    /// Create a `Dish` socket from a specific context.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`SocketLimit`]
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
    pub fn with_ctx(ctx: Ctx) -> Result<Self, Error> {
        let inner = Arc::new(RawSocket::with_ctx(RawSocketType::Dish, ctx)?);

        Ok(Self { inner })
    }

    /// Returns a reference to the context of the socket.
    pub fn ctx(&self) -> &crate::Ctx {
        &self.inner.ctx
    }
    /// Joins the specified group.
    ///
    /// # Usage Contract
    /// * The group `str` must be at most 15 characters.
    /// * Each group can be subscribed at most once.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`InvalidInput`] (if contract is not followed)
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    pub fn join<'a, G>(&self, group: G) -> Result<(), Error>
    where
        &'a Group: TryFrom<G> + Sized,
        Error: From<<&'a Group as TryFrom<G>>::Error>,
    {
        let group: &Group = group.try_into()?;
        let c_str = CString::new(group.as_str()).unwrap();
        let rc =
            unsafe { sys::zmq_join(self.mut_raw_socket(), c_str.as_ptr()) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = {
                match errno {
                    errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                        msg: "invalid group",
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

    /// Leave the specified group.
    ///
    /// # Usage Contract
    /// * The group `str` must be at most 15 characters.
    /// * The group must be already joined.
    ///
    /// # Returned Error Variants
    /// * [`CtxTerminated`]
    /// * [`Interrupted`]
    /// * [`InvalidInput`] (if contract is not followed)
    ///
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    pub fn leave<'a, G>(&self, group: G) -> Result<(), Error>
    where
        &'a Group: TryFrom<G>,
        Error: From<<&'a Group as TryFrom<G>>::Error>,
    {
        let group: &Group = group.try_into()?;
        let c_str = CString::new(group.as_str()).unwrap();
        let rc =
            unsafe { sys::zmq_leave(self.mut_raw_socket(), c_str.as_ptr()) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = {
                match errno {
                    errno::EINVAL => panic!("Invalid group"),
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
}

impl GetRawSocket for Dish {
    fn raw_socket(&self) -> *const c_void {
        self.inner.socket
    }

    // This is safe as long as it is only used by libzmq.
    fn mut_raw_socket(&self) -> *mut c_void {
        self.inner.socket as *mut _
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

    pub fn build(&self) -> Result<Dish, Error> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Dish, Error> {
        let dish = Dish::with_ctx(ctx)?;
        self.apply(&dish)?;

        Ok(dish)
    }

    pub fn groups(&self) -> Option<&[GroupOwned]> {
        self.groups.as_ref().map(|g| g.as_slice())
    }

    pub fn set_groups<G>(mut self, maybe_groups: Option<Vec<G>>) where G: Into<GroupOwned> {
        let groups = maybe_groups.map(|g| g.into_iter().map(|g| g.into()).collect());
        self.groups = groups;
    }

    pub fn apply(&self, dish: &Dish) -> Result<(), Error> {
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

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        &mut self.socket_config
    }
}

impl ConfigureSocket for DishConfig {}

impl GetRecvConfig for DishConfig {
    fn recv_config(&self) -> &RecvConfig {
        &self.recv_config
    }

    fn mut_recv_config(&mut self) -> &mut RecvConfig {
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

    pub fn build(&self) -> Result<Dish, Error> {
        self.inner.build()
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Dish, Error> {
        self.inner.build_with_ctx(ctx)
    }
}

impl GetSocketConfig for DishBuilder {
    fn socket_config(&self) -> &SocketConfig {
        self.inner.socket_config()
    }

    fn mut_socket_config(&mut self) -> &mut SocketConfig {
        self.inner.mut_socket_config()
    }
}

impl BuildSocket for DishBuilder {}

impl GetRecvConfig for DishBuilder {
    fn recv_config(&self) -> &RecvConfig {
        self.inner.recv_config()
    }

    fn mut_recv_config(&mut self) -> &mut RecvConfig {
        self.inner.mut_recv_config()
    }
}

impl BuildRecv for DishBuilder {}
