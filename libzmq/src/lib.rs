//! libzmq - A strict subset of ØMQ with a high level API.

mod ctx;
pub mod endpoint;
mod msg;
pub mod socket;
mod sockopt;

pub use ctx::*;
pub use error::*;
pub use msg::*;

use libzmq_sys as sys;

use failure::{Backtrace, Context, Fail};

use std::{
    ffi,
    fmt::{self, Debug, Display},
    os::raw::*,
    str,
};

pub mod prelude {
    pub use crate::{
        ctx::{Ctx, CtxConfig},
        endpoint::Endpoint,
        error::{Error, ErrorKind},
        msg::Msg,
        socket::{Client, Dish, Radio, RecvMsg, SendMsg, Server, Socket},
    };
}

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

mod error {
    use super::*;

    /// An error with a kind and a content.
    ///
    /// An `Error` contains a [`ErrorKind`] which gives context on the error cause,
    /// as well as `Option<T>` which is used to prevent the loss of data
    /// in case of a failed function call.
    ///
    /// # Usage example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::prelude::*;
    /// // This will make our match pattern cleaner.
    /// use ErrorKind::*;
    ///
    /// // This client has no peer and is therefore in mute state.
    /// let client = Client::new()?;
    ///
    /// // This means that the following call would block.
    /// if let Err(mut err) = client.send_poll("msg") {
    ///   match err.kind() {
    ///     // This covers all the possible error scenarios for this socket type.
    ///     // Normally we would process each error differently.
    ///     WouldBlock | CtxTerminated | Interrupted => {
    ///       // Here we get back the message we tried to send.
    ///       let msg = err.content().take().unwrap();
    ///       assert_eq!("msg", msg.to_str()?);
    ///     }
    ///     // Since `ErrorKind` is non-exhaustive, need an
    ///     // extra wildcard arm to account for potential future variants.
    ///     _ => panic!("unhandled error : {}", err),
    ///   }
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`ErrorKind`]: enum.ErrorKind.html
    #[derive(Debug)]
    pub struct Error<T>
    where
        T: 'static + Send + Sync + Debug,
    {
        inner: Context<ErrorKind>,
        content: Option<T>,
    }

    impl<T> Error<T>
    where
        T: 'static + Send + Sync + Debug,
    {
        pub(crate) fn new(kind: ErrorKind) -> Self {
            Self {
                inner: Context::new(kind),
                content: None,
            }
        }

        pub(crate) fn with_content(kind: ErrorKind, content: T) -> Self {
            Self {
                inner: Context::new(kind),
                content: Some(content),
            }
        }

        /// Returns the kind of error.
        pub fn kind(&self) -> ErrorKind {
            *self.inner.get_context()
        }

        /// Returns the content held by the error.
        pub fn content(&self) -> Option<&T> {
            self.content.as_ref()
        }

        /// Takes the content of the error, if any.
        pub fn take_content(&mut self) -> Option<T> {
            self.content.take()
        }
    }

    impl<T> Fail for Error<T>
    where
        T: 'static + Send + Sync + Debug,
    {
        fn cause(&self) -> Option<&Fail> {
            self.inner.cause()
        }

        fn backtrace(&self) -> Option<&Backtrace> {
            self.inner.backtrace()
        }
    }

    impl<T> Display for Error<T>
    where
        T: 'static + Send + Sync + Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            Display::fmt(&self.inner, f)
        }
    }

    /// Used to give context to an `Error`.
    ///
    /// # Note
    /// This error type is non-exhaustive and could have additional variants
    /// added in future. Therefore, when matching against variants of
    /// non-exhaustive enums, an extra wildcard arm must be added to account
    /// for any future variants.
    ///
    /// [`Error`]: enum.Error.html
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Fail)]
    pub enum ErrorKind {
        /// Non-blocking mode was requested and the message cannot be sent
        /// without blocking
        #[fail(display = "operation would block")]
        WouldBlock,
        /// Occurs when a [`Server`] socket cannot route a message
        /// to a host.
        ///
        /// [`Server`]: socket/struct.Server.html
        #[fail(display = "host unreachable")]
        HostUnreachable,
        /// The context was terminated while the operation was ongoing. Any
        /// further operations on sockets that share this context will result
        /// in this error.
        ///
        /// This error can only occur if the [`Ctx`] was explicitely [`terminated`].
        ///
        /// [`Ctx`]: ../ctx/struct.Ctx.html
        /// [`terminated`]: ../ctx/struct.Ctx.html#method.terminate
        #[fail(display = "context terminated")]
        CtxTerminated,
        /// The operation was interrupted by a OS signal delivery.
        #[fail(display = "interrupted by signal")]
        Interrupted,
        /// The addr cannot be bound because it is already in use.
        #[fail(display = "addr in use")]
        AddrInUse,
        /// A nonexistent interface was requested or the requested address was
        /// not local.
        #[fail(display = "addr not available")]
        AddrNotAvailable,
        /// An entity was not found.
        ///
        /// The inner `msg` contains information on the specific entity.
        #[fail(display = "not found: {}", msg)]
        NotFound {
            /// Additionnal information on the error.
            msg: &'static str,
        },
        /// The open socket limit was reached.
        #[fail(display = "open socket limit was reached")]
        SocketLimit,
        /// A fn call did not follow its usage contract and provided invalid inputs.
        ///
        /// An `InvalidInput` error is guaranteed to be related to some API misuse
        /// that can be known at compile time. Thus `panic` should be called on
        /// those types of error.
        ///
        /// The inner `msg` contains information on the specific contract breach.
        #[fail(display = "invalid input: {}", msg)]
        InvalidInput {
            /// Additionnal information on the error.
            msg: &'static str,
        },
    }

    pub(crate) fn msg_from_errno(x: i32) -> String {
        unsafe {
            let s = sys::zmq_strerror(x);
            format!(
                "unknown error [{}]: {}",
                x,
                str::from_utf8(ffi::CStr::from_ptr(s).to_bytes()).unwrap()
            )
        }
    }
}
