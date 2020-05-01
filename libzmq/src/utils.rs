use crate::{core::GetRawSocket, error::*};
use libzmq_sys as sys;
use sys::errno;

use std::{os::raw::*, ptr};

/// Reports the ØMQ library version.
///
/// Returns a tuple in the format `(Major, Minor, Patch)`.
///
/// See [`zmq_version`].
///
/// [`zmq_version`]: http://api.zeromq.org/4-2:zmq-version
///
/// ```
/// use libzmq::version;
///
/// assert_eq!(version(), (4, 3, 2));
/// ```
// This test acts as a canary when upgrading the *libzmq*
// version.
pub fn version() -> (i32, i32, i32) {
    let mut major = 0;
    let mut minor = 0;
    let mut patch = 0;
    unsafe {
        sys::zmq_version(
            &mut major as *mut c_int,
            &mut minor as *mut c_int,
            &mut patch as *mut c_int,
        );
    }
    (major, minor, patch)
}

/// Start a built-in ØMQ proxy between a frontend and a backend socket.
///
/// The two sockets must be configured before creating the proxy.
///
/// The proxy connects a frontend socket to a backend socket. Conceptually, data
/// flows from frontend to backend. Depending on the socket types, replies may
/// flow in the opposite direction. The direction is conceptual only; the proxy
/// is fully symmetric and there is no technical difference between frontend and
/// backend.
///
/// # Returned Errors
/// * [`InvalidCtx`]
///
/// # Example
/// ```
/// # fn main() -> Result<(), anyhow::Error> {
/// use libzmq::{prelude::*, *};
/// use std::thread;
///
/// let radio_addr: InprocAddr = "frontend".try_into()?;
/// let dish_addr: InprocAddr = "backend".try_into()?;
/// let group: Group = "some group".try_into()?;
///
/// let radio = RadioBuilder::new()
///     .bind(&radio_addr)
///     .build()?;
///
/// let frontend = DishBuilder::new()
///     .connect(&radio_addr)
///     .join(&group)
///     .build()?;
///
/// let backend = RadioBuilder::new()
///     .bind(&dish_addr)
///     .build()?;
///
/// let dish = DishBuilder::new()
///     .connect(&dish_addr)
///     .join(&group)
///     .build()?;
///
/// let proxy_handle = thread::spawn(move || proxy(frontend, backend));
///
/// let mut msg = Msg::new();
/// msg.set_group(&group);
/// radio.send(msg)?;
///
/// let msg = dish.recv_msg()?;
/// assert!(msg.is_empty());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
pub fn proxy<F, B>(frontend: F, backend: B) -> Result<(), Error>
where
    F: GetRawSocket,
    B: GetRawSocket,
{
    let frontend_socket_ptr = frontend.raw_socket().as_mut_ptr();
    let backend_socket_ptr = backend.raw_socket().as_mut_ptr();
    let rc = unsafe {
        sys::zmq_proxy(frontend_socket_ptr, backend_socket_ptr, ptr::null_mut())
    };

    assert_eq!(rc, -1);

    let errno = unsafe { sys::zmq_errno() };
    let err = match errno {
        errno::ETERM => Error::new(ErrorKind::InvalidCtx),
        _ => panic!(msg_from_errno(errno)),
    };

    Err(err)
}
