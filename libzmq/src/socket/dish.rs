use crate::{Ctx, socket::{*, raw::*, config::*}, error::*};

use libzmq_sys as sys;
use sys::errno;

#[macro_use]
use super::macros::*;

use serde::{Serialize, Deserialize};

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
    impl_socket_methods!(Dish);

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
    pub fn join<S>(&self, group: S) -> Result<(), Error<()>>
    where
        S: AsRef<str>,
    {
        let c_str = CString::new(group.as_ref()).unwrap();
        let rc =
            unsafe { sys::zmq_join(self.as_mut_raw_socket(), c_str.as_ptr()) };

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
    pub fn leave<S>(&self, group: S) -> Result<(), Error<()>>
    where
        S: AsRef<str>,
    {
        let c_str = CString::new(group.as_ref()).unwrap();
        let rc =
            unsafe { sys::zmq_leave(self.as_mut_raw_socket(), c_str.as_ptr()) };

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
}

impl_as_raw_socket_trait!(Dish);

impl RecvMsg for Dish {}

unsafe impl Send for Dish {}
unsafe impl Sync for Dish {}

/// A builder for a `Dish`.
///
/// Especially helpfull in config files.
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DishConfig {
    inner: SocketConfig,
    groups: Option<Vec<String>>,
}

impl DishConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Dish, Error<()>> {
        let ctx = Ctx::global().clone();

        self.build_with_ctx(ctx)
    }

    pub fn build_with_ctx(&self, ctx: Ctx) -> Result<Dish, Error<()>> {
        let dish = Dish::with_ctx(ctx)?;
        self.apply(&dish)?;

        if let Some(ref groups) = self.groups {
            for group in groups {
                dish.join(group)?;
            }
        }
        Ok(dish)
    }
}

impl_config_trait!(DishConfig);
