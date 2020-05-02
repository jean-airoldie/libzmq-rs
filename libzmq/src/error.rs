use crate::{addr::AddrParseError, group::GroupParseError};
use libzmq_sys as sys;
use thiserror::Error;

use std::{convert::Infallible, ffi, fmt, io, str};

/// An error with a kind and a msg.
///
/// An `Error` contains a [`ErrorKind`] which gives context on the error cause,
/// as well as an `Option<T>` which may be used to prevent the loss of data
/// in case of a failed `send` function call. When no `T` is specified, it
/// defaults to `()`.
///
/// # Usage example
/// ```
/// # fn main() -> Result<(), anyhow::Error> {
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
///         WouldBlock | InvalidCtx | Interrupted => {
///             // Here we get back the message we tried to send.
///             let msg = err.take().unwrap();
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
pub struct Error<T = ()> {
    kind: ErrorKind,
    content: Option<T>,
}

impl<T> Error<T> {
    /// Creates a new `Error` from an `ErrorKind`.
    ///
    /// The `content` field will be `None`.
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            content: None,
        }
    }

    /// Creates a new `Error` from an `ErrorKind` and some content.
    pub(crate) fn with_content(kind: ErrorKind, content: T) -> Self {
        Self {
            kind,
            content: Some(content),
        }
    }

    /// Returns the kind of error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    #[deprecated(since = "0.2.1", note = "please use `get` instead")]
    pub fn content(&self) -> Option<&T> {
        self.content.as_ref()
    }

    /// Returns a reference to the content held by the error.
    pub fn get(&self) -> Option<&T> {
        self.content.as_ref()
    }

    #[deprecated(since = "0.2.1", note = "please use `take` instead")]
    pub fn take_content(&mut self) -> Option<T> {
        self.content.take()
    }

    /// Takes the content held by the error, if any, replacing with `None`.
    pub fn take(&mut self) -> Option<T> {
        self.content.take()
    }

    /// This allows casting to any `Error<I>` by replacing the content
    /// of the error with `None`.
    ///
    /// This is not implemented as `Into<Error<I>>` to be explicit since
    /// information is lost in the conversion.
    pub fn cast<I>(self) -> Error<I> {
        Error {
            kind: self.kind,
            content: None,
        }
    }
}

impl<T> std::error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.kind.source()
    }
}

impl<T> fmt::Debug for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error").field("kind", &self.kind).finish()
    }
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl<T> From<GroupParseError> for Error<T> {
    fn from(_error: GroupParseError) -> Self {
        Error::new(ErrorKind::InvalidInput("unable to parse group"))
    }
}

impl<T> From<AddrParseError> for Error<T> {
    fn from(error: AddrParseError) -> Self {
        Error::new(ErrorKind::InvalidInput(error.msg()))
    }
}

impl<T> From<Infallible> for Error<T> {
    fn from(_error: Infallible) -> Self {
        unreachable!()
    }
}

impl<T> From<Error<T>> for io::Error {
    fn from(err: Error<T>) -> io::Error {
        use ErrorKind::*;
        match err.kind() {
            WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
            HostUnreachable => {
                io::Error::new(io::ErrorKind::BrokenPipe, "host unreachable")
            }
            InvalidCtx => {
                io::Error::new(io::ErrorKind::Other, "context terminated")
            }
            Interrupted => io::Error::from(io::ErrorKind::Interrupted),
            AddrInUse => io::Error::from(io::ErrorKind::AddrInUse),
            AddrNotAvailable => {
                io::Error::from(io::ErrorKind::AddrNotAvailable)
            }
            NotFound(msg) => io::Error::new(io::ErrorKind::NotFound, msg),
            SocketLimit => {
                io::Error::new(io::ErrorKind::Other, "socket limit reached")
            }
            InvalidInput(msg) => {
                io::Error::new(io::ErrorKind::InvalidInput, msg)
            }
        }
    }
}

/// Used to give context to an `Error`.
///
/// [`Error`]: enum.Error.html
#[derive(Debug, Error, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Non-blocking mode was requested and the message cannot be sent
    /// without blocking
    #[error("operation would block")]
    WouldBlock,
    /// Occurs when a [`Server`] socket cannot route a message
    /// to a host.
    ///
    /// [`Server`]: socket/struct.Server.html
    #[error("host unreachable")]
    HostUnreachable,
    /// The context used in the operation was invalidated. Either the
    /// context is being terminated, or was already terminated.
    ///
    /// This error only occurs if:
    /// * The `Ctx` is being dropped or was previously dropped.
    /// * [`shutdown`] was called.
    ///
    /// [`Ctx`]: ../ctx/struct.Ctx.html
    /// [`shutdown`]: ../ctx/struct.Ctx.html#method.terminate
    #[error("context invalidated")]
    InvalidCtx,
    /// The operation was interrupted by a OS signal delivery.
    #[error("interrupted by signal")]
    Interrupted,
    /// The addr cannot be bound because it is already in use.
    #[error("addr in use")]
    AddrInUse,
    /// A nonexistent interface was requested or the requested address was
    /// not local.
    #[error("addr not available")]
    AddrNotAvailable,
    /// An entity was not found.
    ///
    /// Contains information on the specific entity.
    #[error("not found: {}", _0)]
    NotFound(&'static str),
    /// The open socket limit was reached.
    #[error("open socket limit was reached")]
    SocketLimit,
    /// The user did not follow its usage contract and provided invalid inputs.
    ///
    /// Contains information on the specific contract breach.
    #[error("invalid input: {}", _0)]
    InvalidInput(&'static str),
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
