//! The ØMQ context type.

use crate::{auth::server::AuthServer, error::*};
use libzmq_sys as sys;
use sys::errno;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use std::{
    os::raw::{c_int, c_void},
    str, thread,
};

lazy_static! {
    static ref GLOBAL_CONTEXT: Ctx = Ctx::new();
}

#[derive(Copy, Clone, Debug)]
enum CtxOption {
    IOThreads,
    MaxSockets,
    MaxMsgSize,
    SocketLimit,
    IPV6,
    Blocky,
}

impl From<CtxOption> for c_int {
    fn from(r: CtxOption) -> c_int {
        match r {
            CtxOption::IOThreads => sys::ZMQ_IO_THREADS as c_int,
            CtxOption::MaxSockets => sys::ZMQ_MAX_SOCKETS as c_int,
            CtxOption::MaxMsgSize => sys::ZMQ_MAX_MSGSZ as c_int,
            CtxOption::SocketLimit => sys::ZMQ_SOCKET_LIMIT as c_int,
            CtxOption::IPV6 => sys::ZMQ_IPV6 as c_int,
            CtxOption::Blocky => sys::ZMQ_BLOCKY as c_int,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RawCtx {
    ctx: *mut c_void,
}

impl RawCtx {
    fn new() -> Self {
        let ctx = unsafe { sys::zmq_ctx_new() };

        if ctx.is_null() {
            panic!(msg_from_errno(unsafe { sys::zmq_errno() }));
        }

        Self { ctx }
    }

    fn get(self, option: CtxOption) -> i32 {
        unsafe { sys::zmq_ctx_get(self.ctx, option.into()) }
    }

    fn set(self, option: CtxOption, value: i32) -> Result<(), Error> {
        let rc = unsafe { sys::zmq_ctx_set(self.ctx, option.into(), value) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            match errno {
                errno::EINVAL => {
                    Err(Error::new(ErrorKind::InvalidInput("invalid value")))
                }
                _ => panic!(msg_from_errno(errno)),
            }
        } else {
            Ok(())
        }
    }

    fn set_bool(self, opt: CtxOption, flag: bool) -> Result<(), Error> {
        self.set(opt, flag as i32)
    }

    fn terminate(self) {
        // We loop in case `zmq_ctx_term` get interrupted by a signal.
        loop {
            let rc = unsafe { sys::zmq_ctx_term(self.ctx) };
            if rc == 0 {
                break;
            } else {
                let errno = unsafe { sys::zmq_errno() };
                match errno {
                    errno::EINTR => (),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn shutdown(self) {
        let rc = unsafe { sys::zmq_ctx_shutdown(self.ctx) };
        // Should never fail.
        assert_eq!(rc, 0);
    }
}

// The `zmq_ctx` is internally threadsafe.
unsafe impl Send for RawCtx {}
unsafe impl Sync for RawCtx {}

/// A config for a [`Ctx`].
///
/// Usefull in configuration files.
///
/// [`Ctx`]: struct.Ctx.html
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CtxConfig {
    io_threads: Option<i32>,
    max_sockets: Option<i32>,
}

impl CtxConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<Ctx, Error> {
        let ctx = Ctx::new();
        self.apply(ctx.handle())?;

        Ok(ctx)
    }

    pub fn apply(&self, handle: CtxHandle) -> Result<(), Error> {
        if let Some(value) = self.io_threads {
            handle.set_io_threads(value)?;
        }
        if let Some(value) = self.max_sockets {
            handle.set_max_sockets(value)?;
        }

        Ok(())
    }

    pub fn io_threads(&self) -> Option<i32> {
        self.io_threads
    }

    pub fn set_io_threads(&mut self, value: Option<i32>) {
        self.io_threads = value;
    }

    pub fn max_sockets(&mut self) -> Option<i32> {
        self.max_sockets
    }

    pub fn set_max_sockets(&mut self, value: Option<i32>) {
        self.max_sockets = value;
    }
}

/// A convenience builder for a [`Ctx`].
///
/// Makes complex context configuration more convenient.
///
/// [`Ctx`]: struct.Ctx.html
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CtxBuilder {
    inner: CtxConfig,
}

impl CtxBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a `Ctx` from a `CtxBuilder`.
    ///
    /// # Usage Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::*;
    ///
    /// let ctx = CtxBuilder::new()
    ///   .io_threads(2)
    ///   .build()?;
    ///
    /// assert_eq!(ctx.io_threads(), 2);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn build(&self) -> Result<Ctx, Error> {
        let ctx = Ctx::new();
        self.apply(ctx.handle())?;

        Ok(ctx)
    }

    /// Applies the configuration of `CtxBuilder` to an existing context via
    /// its `CtxHandle`.
    ///
    /// # Usage Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::*;
    ///
    /// let global = Ctx::global();
    ///
    /// CtxBuilder::new()
    ///   .io_threads(0)
    ///   .max_sockets(69)
    ///   .apply(global)?;
    ///
    /// assert_eq!(global.io_threads(), 0);
    /// assert_eq!(global.max_sockets(), 69);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn apply(&self, handle: CtxHandle) -> Result<(), Error> {
        self.inner.apply(handle)
    }

    /// See [`set_io_threads`].
    ///
    /// [`set_io_threads`]: struct.Ctx.html#method.set_io_threads
    pub fn io_threads(&mut self, value: i32) -> &mut Self {
        self.inner.set_io_threads(Some(value));
        self
    }

    /// See [`set_max_sockets`].
    ///
    /// [`set_max_sockets`]: struct.Ctx.html#method.set_max_sockets
    pub fn max_sockets(&mut self, value: i32) -> &mut Self {
        self.inner.set_max_sockets(Some(value));
        self
    }
}

/// A non-owning pointer to a `Ctx`.
///
/// A `CtxHandle` allows thread-safe configuration of the context aliased by
/// the handle. It is also used to created sockets associated with the context.
///
/// Once the `Ctx` it is pointing to is `shutdown` or dropped, all associated
/// `CtxHandle` will be invalidated. All calls involving an invalidated
/// `CtxHandle` will return a `CtxInvalid` error.
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{Ctx, Dish, ErrorKind};
///
/// // We create a `CtxHandle` from a new context. Since we drop
/// // the context pointed by the handle, it will no longer be valid
/// // once it reaches the outer scope.
/// let handle = {
///     let ctx = Ctx::new();
///     ctx.handle()
/// };
///
/// // Attempting to use the invalided handle will result in `CtxInvalid`
/// // errors.
/// let err = Dish::with_ctx(handle).unwrap_err();
/// match err.kind() {
///     ErrorKind::CtxInvalid => (),
///     _ => unreachable!(),
/// }
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct CtxHandle {
    inner: RawCtx,
}

impl CtxHandle {
    /// [`Read more`](struct.Ctx.html#method.io_threads)
    pub fn io_threads(self) -> i32 {
        self.inner.get(CtxOption::IOThreads)
    }

    /// [`Read more`](struct.Ctx.html#method.set_io_threads)
    pub fn set_io_threads(self, nb_threads: i32) -> Result<(), Error> {
        self.inner.set(CtxOption::IOThreads, nb_threads)
    }

    /// [`Read more`](struct.Ctx.html#method.max_sockets)
    pub fn max_sockets(self) -> i32 {
        self.inner.get(CtxOption::MaxSockets)
    }

    /// [`Read more`](struct.Ctx.html#method.set_max_sockets)
    pub fn set_max_sockets(self, max: i32) -> Result<(), Error> {
        self.inner.set(CtxOption::MaxSockets, max)
    }

    /// [`Read more`](struct.Ctx.html#method.shutdown)
    pub fn shutdown(self) {
        self.inner.shutdown()
    }

    pub(crate) fn as_ptr(self) -> *mut c_void {
        self.inner.ctx
    }
}

/// A owning pointer to a ØMQ context.
///
/// A context leeps the list of sockets and manages the async I/O thread and
/// internal queries.
///
/// Each context also has an associated `AuthServer` which handles socket
/// authentification.
///
/// # Drop Behavior
/// The context will call terminate when dropped which will cause all
/// blocking calls to fail with `CtxInvalid`, then the dropping thread
/// will block until the following conditions are met:
/// * All sockets open within the context have been dropped.
/// * All messages within the context are closed.
///
/// To prevent the context drop from blocking infinitely, users should properly
/// manage `Result` returned by function calls.
///
/// # Thread safety
/// A ØMQ context is internally thread safe.
#[derive(Eq, PartialEq, Debug)]
pub struct Ctx {
    inner: RawCtx,
}

impl Ctx {
    /// Create a new ØMQ context.
    ///
    /// For almost all use cases, using and configuring the [`global`] context
    /// will be enought.
    ///
    /// See [`zmq_ctx_new`].
    ///
    /// [`zmq_ctx_new`]: http://api.zeromq.org/master:zmq-ctx-new
    ///
    /// # Usage Example
    /// ```
    /// use libzmq::Ctx;
    ///
    /// // Creates a new unique context.
    /// let ctx = Ctx::new();
    /// // Returns a handle to the context that can be used
    /// // to create sockets.
    /// let handle = ctx.handle();
    /// ```
    ///
    /// [`global`]: #method.global
    pub fn new() -> Self {
        let inner = RawCtx::new();
        // Enable ipv6 by default.
        inner.set_bool(CtxOption::IPV6, true).unwrap();
        // Set linger period for all sockets to zero.
        inner.set_bool(CtxOption::Blocky, false).unwrap();

        //// Start a `ZAP` handler for the context.
        let mut auth = AuthServer::with_ctx(CtxHandle { inner }).unwrap();

        // This thread is guaranteed to terminate with the ctx because
        // it terminates on `CtxInvalid` errors.
        thread::spawn(move || auth.run());

        Self { inner }
    }

    /// Returns a handle to the `Ctx`.
    ///
    /// Sockets can be created using `CtxHandle` so that they used the
    /// context aliased by the handle.
    ///
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{Ctx, Server};
    ///
    /// let ctx = Ctx::new();
    /// let handle = ctx.handle();
    ///
    /// let server = Server::with_ctx(handle)?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn handle(&self) -> CtxHandle {
        CtxHandle { inner: self.inner }
    }

    /// Returns a handle to the global context.
    ///
    /// This is a singleton used by sockets created via their respective
    /// `::new()` method. It merely exists for convenience and is no different
    /// from a context obtained via `Ctx::new()`.
    ///
    /// # Usage Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{Ctx, Client};
    ///
    /// // A socket created via `new` will use the global context via
    /// // its `CtxHandle`.
    /// let client = Client::new()?;
    /// assert_eq!(client.ctx(), Ctx::global());
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn global() -> CtxHandle {
        GLOBAL_CONTEXT.handle()
    }

    /// Returns the size of the ØMQ thread pool for this context.
    pub fn io_threads(&self) -> i32 {
        self.inner.get(CtxOption::IOThreads)
    }

    /// Set the size of the ØMQ thread pool to handle I/O operations.
    ///
    /// "The general rule of thumb is to allow one I/O thread per gigabyte of
    /// data in or out per second." - [`Pieter Hintjens`]
    ///
    /// [`Pieter Hintjens`]: http://zguide.zeromq.org/page:all#I-O-Threads
    ///
    /// # Default
    /// The default value is `1`.
    ///
    /// # Usage Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::Ctx;
    ///
    /// let ctx = Ctx::new();
    /// assert_eq!(ctx.io_threads(), 1);
    ///
    /// // Lets say our app exclusively uses the inproc transport
    /// // for messaging. Then we dont need any I/O threads.
    /// ctx.set_io_threads(0)?;
    /// assert_eq!(ctx.io_threads(), 0);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_io_threads(&self, nb_threads: i32) -> Result<(), Error> {
        self.inner.set(CtxOption::IOThreads, nb_threads)
    }

    /// Returns the maximum number of sockets allowed for this context.
    pub fn max_sockets(&self) -> i32 {
        self.inner.get(CtxOption::MaxSockets)
    }

    /// Sets the maximum number of sockets allowed on the context.
    ///
    /// # Default
    /// The default value is `1023`.
    ///
    /// # Usage Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::Ctx;
    ///
    /// let ctx = Ctx::new();
    /// assert_eq!(ctx.max_sockets(), 1023);
    ///
    /// ctx.set_max_sockets(420)?;
    /// assert_eq!(ctx.max_sockets(), 420);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_max_sockets(&self, max: i32) -> Result<(), Error> {
        self.inner.set(CtxOption::MaxSockets, max)
    }

    /// Returns the largest number of sockets that the context will accept.
    pub fn socket_limit(&self) -> i32 {
        self.inner.get(CtxOption::SocketLimit)
    }

    /// Invalidates all the handles to the ØMQ context.
    ///
    /// Context shutdown will cause any blocking operations currently in
    /// progress on sockets using handles associated with the context to fail
    /// with [`CtxInvalid`].
    ///
    /// This is used as a mechanism to stop another blocked thread.
    ///
    /// Note that, while this invalidates the context, it does not terminate it.
    /// The context will only get terminated once `Ctx` is dropped.
    ///
    /// [`CtxInvalid`]: ../error/enum.ErrorKind.html#variant.CtxInvalid
    pub fn shutdown(&self) {
        self.inner.shutdown()
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Ctx {
    fn drop(&mut self) {
        self.inner.terminate()
    }
}
