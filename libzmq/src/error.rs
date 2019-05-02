use crate::{endpoint::EndpointParseError, group::GroupError, Msg};
use libzmq_sys as sys;

use failure::{Backtrace, Context, Fail};

use std::{
    ffi,
    fmt::{self, Display},
    str,
    convert::Infallible,
};

/// An error with a kind and a msg.
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
/// use libzmq::{prelude::*, *, ErrorKind::*};
///
/// // This client has no peer and is therefore in mute state.
/// let client = Client::new()?;
///
/// // This means that the following call would block.
/// if let Err(mut err) = client.try_send("msg") {
///     match err.kind() {
///         // This covers all the possible error scenarios for this socket type.
///         // Normally we would process each error differently.
///         WouldBlock | CtxTerminated | Interrupted => {
///             // Here we get back the message we tried to send.
///             let msg = err.take_msg().unwrap();
///             assert_eq!("msg", msg.to_str()?);
///         }
///         // Since `ErrorKind` is non-exhaustive, need an
///         // extra wildcard arm to account for potential future variants.
///         _ => panic!("unhandled error : {}", err),
///     }
/// }
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`ErrorKind`]: enum.ErrorKind.html
#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
    msg: Option<Msg>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            inner: Context::new(kind),
            msg: None,
        }
    }

    pub(crate) fn with_msg(kind: ErrorKind, msg: Msg) -> Self {
        Self {
            inner: Context::new(kind),
            msg: Some(msg),
        }
    }

    /// Returns the kind of error.
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }

    /// Returns a reference to the msg held by the error.
    pub fn msg(&self) -> Option<&Msg> {
        self.msg.as_ref()
    }

    /// Takes the msg held by the error, if any.
    pub fn take_msg(&mut self) -> Option<Msg> {
        self.msg.take()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl From<EndpointParseError> for Error {
    fn from(error: EndpointParseError) -> Self {
        Error::new(ErrorKind::InvalidInput {
            msg: "invalid endpoint",
        })
    }
}

impl From<GroupError> for Error {
    fn from(error: GroupError) -> Self {
        Error::new(ErrorKind::InvalidInput {
            msg: "invalid group",
        })
    }
}

impl From<Infallible> for Error {
    fn from(error: Infallible) -> Self {
        unreachable!()
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Fail, Hash)]
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
