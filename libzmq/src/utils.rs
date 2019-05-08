use crate::{core::GetRawSocket, error::*};
use libzmq_sys as sys;
use sys::errno;

use std::{ffi, os::raw::*, ptr, str};

/// Reports the ØMQ library version.
///
/// Returns a tuple in the format `(Major, Minor, Patch)`.
///
/// See [`zmq_version`].
///
/// [`zmq_version`]: http://api.zeromq.org/4-2:zmq-version
///
/// ```
/// use libzmq::zmq_version;
///
/// assert_eq!(zmq_version(), (4, 3, 1));
/// ```
// This test acts as a canary when upgrading the libzmq
// version.
pub fn zmq_version() -> (i32, i32, i32) {
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

/// Check for a ZMQ capability.
///
/// See [`zmq_has`].
///
/// [`zmq_has`]: http://api.zeromq.org/4-2:zmq-has
///
/// ```
/// use libzmq::zmq_has;
///
/// assert!(zmq_has("curve"));
/// ```
pub fn zmq_has(capability: &str) -> bool {
    let c_str = ffi::CString::new(capability).unwrap();
    unsafe { sys::zmq_has(c_str.as_ptr()) == 1 }
}

/// # Returned Errors
/// * [`CtxTerminated`]
///
/// # Example
/// ```
/// #
/// # use failure::Error;
/// # fn main() -> Result<(), Error> {
/// use libzmq::{
///     prelude::*, Ctx, addr::InprocAddr, socket::*, Group, ErrorKind, Msg, proxy
/// };
/// use std::{thread, convert::TryInto};
///
/// let radio_addr: InprocAddr = "frontend".try_into()?;
/// let dish_addr: InprocAddr = "backend".try_into()?;
/// let group: &Group = "some group".try_into()?;
///
/// // Using `no_drop = true` is usually an anti-pattern. In this case it is used
/// // for demonstration purposes.
/// let radio = RadioBuilder::new()
///     .no_drop()
///     .bind(&radio_addr)
///     .build()?;
///
/// let frontend = DishBuilder::new()
///     .connect(&radio_addr)
///     .join(group)
///     .build()?;
///
/// let backend = RadioBuilder::new()
///     .bind(&dish_addr)
///     .build()?;
///
/// let dish = DishBuilder::new()
///     .connect(&dish_addr)
///     .join(group)
///     .build()?;
///
/// let proxy_handle = thread::spawn(move || proxy(&frontend, &backend));
///
/// let mut msg = Msg::new();
/// msg.set_group(group);
/// radio.send(msg).unwrap();
/// let msg = dish.recv_msg().unwrap();
/// assert!(msg.is_empty());
///
/// // This will cause the proxy to error out with `CtxTerminated`.
/// let _ = Ctx::global().shutdown();
/// let err = proxy_handle.join().unwrap().unwrap_err();
/// assert_eq!(err.kind(), ErrorKind::CtxTerminated);
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
pub fn proxy<F, B>(frontend: &F, backend: &B) -> Result<(), Error>
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
    let err = {
        match errno {
            errno::ETERM => Error::new(ErrorKind::CtxTerminated),
            _ => panic!(msg_from_errno(errno)),
        }
    };

    Err(err)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("../README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
