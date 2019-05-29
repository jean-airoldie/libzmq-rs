//! The ØMQ context type.

use crate::{auth::server::AuthServer, error::msg_from_errno};
use libzmq_sys as sys;
use sys::errno;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use std::{
    os::raw::{c_int, c_void},
    ptr, str,
    sync::Arc,
    thread,
};

lazy_static! {
    static ref GLOBAL_CONTEXT: Ctx = Ctx::new();
}

#[derive(Copy, Clone, Debug)]
enum RawCtxOption {
    IOThreads,
    MaxSockets,
    MaxMsgSize,
    SocketLimit,
    IPV6,
    Blocky,
}

impl From<RawCtxOption> for c_int {
    fn from(r: RawCtxOption) -> c_int {
        match r {
            RawCtxOption::IOThreads => sys::ZMQ_IO_THREADS as c_int,
            RawCtxOption::MaxSockets => sys::ZMQ_MAX_SOCKETS as c_int,
            RawCtxOption::MaxMsgSize => sys::ZMQ_MAX_MSGSZ as c_int,
            RawCtxOption::SocketLimit => sys::ZMQ_SOCKET_LIMIT as c_int,
            RawCtxOption::IPV6 => sys::ZMQ_IPV6 as c_int,
            RawCtxOption::Blocky => sys::ZMQ_BLOCKY as c_int,
        }
    }
}

#[derive(Debug)]
struct RawCtx {
    ctx: *mut c_void,
}

impl RawCtx {
    fn get(&self, option: RawCtxOption) -> Option<i32> {
        let value = unsafe { sys::zmq_ctx_get(self.ctx, option.into()) };
        if value == -1 {
            None
        } else {
            Some(value)
        }
    }

    // The `zmq_ctx` is already thread safe, so no need to make this mutable.
    fn set(&self, option: RawCtxOption, value: i32) {
        let rc = unsafe { sys::zmq_ctx_set(self.ctx, option.into(), value) };
        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            panic!(msg_from_errno(errno));
        }
    }

    fn terminate(&self) {
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

    fn shutdown(&self) {
        let rc = unsafe { sys::zmq_ctx_shutdown(self.ctx) };
        // Should never fail.
        assert_eq!(rc, 0);
    }
}

// The `zmq_ctx` is internally threadsafe.
unsafe impl Send for RawCtx {}
unsafe impl Sync for RawCtx {}

impl Drop for RawCtx {
    fn drop(&mut self) {
        self.terminate()
    }
}

impl PartialEq for RawCtx {
    /// Compares the two underlying raw C pointers.
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.ctx, other.ctx)
    }
}

impl Eq for RawCtx {}

impl Default for RawCtx {
    fn default() -> Self {
        let ctx = unsafe { sys::zmq_ctx_new() };

        if ctx.is_null() {
            panic!(msg_from_errno(unsafe { sys::zmq_errno() }));
        }

        Self { ctx }
    }
}

/// A config for a [`Ctx`].
///
/// Usefull in configuration files.
///
/// [`Ctx`]: struct.Ctx.html
#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CtxConfig {
    io_threads: Option<i32>,
    max_msg_size: Option<i32>,
    max_sockets: Option<i32>,
    no_linger: Option<bool>,
}

impl CtxConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self, ctx: &Ctx) {
        if let Some(value) = self.io_threads {
            ctx.set_io_threads(value);
        }
        if let Some(value) = self.max_sockets {
            ctx.set_max_sockets(value);
        }
        if let Some(value) = self.max_msg_size {
            ctx.set_max_msg_size(value);
        }
        if let Some(value) = self.no_linger {
            ctx.set_no_linger(value);
        }
    }

    pub fn io_threads(&self) -> Option<i32> {
        self.io_threads
    }

    pub fn set_io_threads(&mut self, value: Option<i32>) {
        self.io_threads = value;
    }

    pub fn max_msg_size(&self) -> Option<i32> {
        self.max_msg_size
    }

    pub fn set_max_msg_size(&mut self, value: Option<i32>) {
        self.max_msg_size = value;
    }

    pub fn max_sockets(&mut self) -> Option<i32> {
        self.max_sockets
    }

    pub fn set_max_sockets(&mut self, value: Option<i32>) {
        self.max_sockets = value;
    }

    pub fn no_linger(&self) -> Option<bool> {
        self.no_linger
    }

    pub fn set_no_linger(&mut self, value: Option<bool>) {
        self.no_linger = value;
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
    /// use libzmq::*;
    ///
    /// let ctx = CtxBuilder::new()
    ///   .io_threads(2)
    ///   .no_linger(true)
    ///   .build();
    ///
    /// assert_eq!(ctx.io_threads(), 2);
    /// assert_eq!(ctx.no_linger(), true);
    /// ```
    pub fn build(&self) -> Ctx {
        let ctx = Ctx::new();
        self.apply(&ctx);

        ctx
    }

    /// Applies a `CtxBuilder` to an existing `Ctx`.
    ///
    /// # Usage Example
    /// ```
    /// use libzmq::*;
    ///
    /// let global = Ctx::global();
    ///
    /// CtxBuilder::new()
    ///   .io_threads(0)
    ///   .max_msg_size(420)
    ///   .max_sockets(69)
    ///   .no_linger(true)
    ///   .apply(global);
    ///
    /// assert_eq!(global.io_threads(), 0);
    /// assert_eq!(global.max_msg_size(), 420);
    /// assert_eq!(global.no_linger(), true);
    /// assert_eq!(global.max_sockets(), 69);
    /// ```
    pub fn apply(&self, ctx: &Ctx) {
        self.inner.apply(ctx);
    }

    /// See [`set_io_threads`].
    ///
    /// [`set_io_threads`]: struct.Ctx.html#method.set_io_threads
    pub fn io_threads(&mut self, value: i32) -> &mut Self {
        self.inner.set_io_threads(Some(value));
        self
    }

    /// See [`set_max_msg_size`].
    ///
    /// [`set_max_msg_size`]: struct.Ctx.html#method.set_max_msg_size
    pub fn max_msg_size(&mut self, value: i32) -> &mut Self {
        self.inner.set_max_msg_size(Some(value));
        self
    }

    /// See [`set_max_sockets`].
    ///
    /// [`set_max_sockets`]: struct.Ctx.html#method.set_max_sockets
    pub fn max_sockets(&mut self, value: i32) -> &mut Self {
        self.inner.set_max_sockets(Some(value));
        self
    }

    /// See [`set_no_linger`].
    ///
    /// [`set_no_linger`]: struct.Ctx.html#method.set_no_linger
    pub fn no_linger(&mut self) -> &mut Self {
        self.inner.set_no_linger(Some(true));
        self
    }
}

/// Keeps the list of sockets and manages the async I/O thread and
/// internal queries.
///
/// Each context also has an associated `AuthServer` which handles socket
/// authentification.
///
/// # Drop
/// The context will call terminate when dropped which will cause all
/// blocking calls to fail with `CtxTerminated`, then block until
/// the following conditions are met:
/// * All sockets open within context have been dropped.
/// * All messages sent by the application with have either been physically
///     transferred to a network peer, or the socket's linger period has expired.
///
/// # Thread safety
/// A ØMQ context is internally thread safe.
///
/// # Multiple Contexts
/// Multiple contexts are allowed but are considered exotic.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Ctx {
    raw: Arc<RawCtx>,
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
    /// let ctx = Ctx::new();
    /// let cloned = ctx.clone();
    ///
    /// assert_eq!(ctx, cloned);
    /// assert_ne!(ctx, Ctx::new());
    /// ```
    ///
    /// [`global`]: #method.global
    pub fn new() -> Self {
        let raw = Arc::new(RawCtx::default());
        // Enable ipv6 by default.
        raw.set(RawCtxOption::IPV6, true as i32);

        let ctx = Self { raw };

        // Start a `ZAP` handler for the context.
        let mut auth = AuthServer::with_ctx(&ctx).unwrap();

        // This thread is guaranteed to terminate before the ctx
        // since it holds a `Arc` to it. No need to store & join the
        // thread handle.
        thread::spawn(move || auth.run());

        ctx
    }

    /// Returns a reference to the global context.
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
    /// // A socket created via `new` will use the global `Ctx`.
    /// let client = Client::new()?;
    /// assert_eq!(client.ctx(), Ctx::global());
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn global() -> &'static Ctx {
        &GLOBAL_CONTEXT
    }

    /// Returns the size of the ØMQ thread pool for this context.
    pub fn io_threads(&self) -> i32 {
        self.raw.as_ref().get(RawCtxOption::IOThreads).unwrap()
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
    /// use libzmq::Ctx;
    ///
    /// let ctx = Ctx::new();
    /// assert_eq!(ctx.io_threads(), 1);
    ///
    /// // Lets say our app exclusively uses the inproc transport
    /// // for messaging. Then we dont need any I/O threads.
    /// ctx.set_io_threads(0);
    /// assert_eq!(ctx.io_threads(), 0);
    /// ```
    pub fn set_io_threads(&self, value: i32) {
        self.raw.as_ref().set(RawCtxOption::IOThreads, value);
    }

    /// Returns the maximum number of sockets allowed for this context.
    pub fn max_sockets(&self) -> i32 {
        self.raw.as_ref().get(RawCtxOption::MaxSockets).unwrap()
    }

    /// Sets the maximum number of sockets allowed on the context.
    ///
    /// # Default
    /// The default value is `1023`.
    ///
    /// # Usage Example
    /// ```
    /// use libzmq::Ctx;
    ///
    /// let ctx = Ctx::new();
    /// assert_eq!(ctx.max_sockets(), 1023);
    ///
    /// ctx.set_max_sockets(420);
    /// assert_eq!(ctx.max_sockets(), 420);
    /// ```
    pub fn set_max_sockets(&self, value: i32) {
        assert!(
            value < self.socket_limit(),
            "cannot be greater than socket limit"
        );
        self.raw.as_ref().set(RawCtxOption::MaxSockets, value);
    }

    /// Returns the maximum size of a message allowed for this context.
    pub fn max_msg_size(&self) -> i32 {
        self.raw.as_ref().get(RawCtxOption::MaxMsgSize).unwrap()
    }

    /// Sets the maximum allowed size of a message sent in the context.
    ///
    /// # Default
    /// The default value is `i32::max_value()`.
    ///
    /// # Usage Example
    /// ```
    /// use libzmq::Ctx;
    ///
    /// let ctx = Ctx::new();
    /// assert_eq!(ctx.max_msg_size(), i32::max_value());
    ///
    /// ctx.set_max_msg_size(i32::max_value() - 1);
    /// assert_eq!(ctx.max_msg_size(), i32::max_value() - 1);
    /// ```
    pub fn set_max_msg_size(&self, value: i32) {
        self.raw.as_ref().set(RawCtxOption::MaxMsgSize, value);
    }

    /// Returns the largest number of sockets that the context will accept.
    pub fn socket_limit(&self) -> i32 {
        self.raw.as_ref().get(RawCtxOption::SocketLimit).unwrap()
    }

    /// A value of `true` indicates that all new sockets are given a
    /// linger timeout of zero.
    ///
    pub fn no_linger(&self) -> bool {
        self.raw.as_ref().get(RawCtxOption::Blocky).unwrap() == 0
    }

    /// When set to `true`, all new sockets are given a linger timeout
    /// of zero.
    ///
    /// # Default
    /// The default value is `false`.
    ///
    /// # Usage Example
    /// ```
    /// use libzmq::Ctx;
    ///
    /// let ctx = Ctx::new();
    /// assert_eq!(ctx.no_linger(), false);
    ///
    /// ctx.set_no_linger(true);
    /// assert_eq!(ctx.no_linger(), true);
    /// ```
    pub fn set_no_linger(&self, enabled: bool) {
        self.raw.as_ref().set(RawCtxOption::Blocky, !enabled as i32);
    }

    /// Shutdown the ØMQ context context.
    ///
    /// Context shutdown will cause any blocking operations currently in
    /// progress on sockets open within context to fail immediately with
    /// [`CtxTerminated`].
    ///
    /// Any further operations on sockets open within context shall fail with
    /// with [`CtxTerminated`].
    ///
    /// [`CtxTerminated`]: ../error/enum.ErrorKind.html#variant.CtxTerminated
    pub fn shutdown(&self) {
        self.raw.shutdown()
    }

    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.raw.ctx
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> From<&'a Ctx> for Ctx {
    fn from(c: &'a Ctx) -> Ctx {
        c.to_owned()
    }
}
