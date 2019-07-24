//! Asynchronous polling mechanim.
//!
//! See the [`Poller`] documentation to get started.
//!
//! [`Poller`]: struct.Poller.html

use crate::{
    core::{AsRawSocket, Period, RawSocket, SocketHandle},
    error::{msg_from_errno, Error, ErrorKind},
    old::OldSocket,
    socket::*,
};
use libzmq_sys as sys;
use sys::errno;

use bitflags::bitflags;

use std::os::{
    raw::{c_short, c_void},
    unix::io::{AsRawFd, RawFd},
};

bitflags! {
    /// A bitflag used to specifies the condition for triggering an [`Event`] in the
    /// [`Poller`].
    ///
    /// # Example
    /// ```
    /// use libzmq::poll::*;
    ///
    /// // This specifies to the poller to trigger an event if `Pollable` is
    /// // readable and/or writable.
    /// let either = READABLE | WRITABLE;
    /// ```
    ///
    /// [`Event`]: struct.Event.html
    /// [`Poller`]: struct.Poller.html
    pub struct Trigger: c_short {
        /// Never trigger.
        const EMPTY = 0i16;
        /// Trigger an `Event` on read readiness.
        ///
        /// For a `libzmq` socket this means that at least one message can be
        /// received without blocking. For a standard socket, this means
        /// that at least one byte of data may be read from the fd without
        /// blocking
        const READABLE = sys::ZMQ_POLLIN as c_short;
        /// Trigger an `Event` on write readiness.
        ///
        /// For a `libzmq` socket this means that at least one message can
        /// be sent without blocking. For a standard socket, this means
        /// that at lesat one bye of data may be written to the fd without
        /// blocking.
        const WRITABLE = sys::ZMQ_POLLOUT as c_short;
    }
}

/// Never trigger an `Event` while polling.
pub const EMPTY: Trigger = Trigger::EMPTY;
/// Trigger an `Event` on read readiness.
///
/// For a `libzmq` socket this means that at least one message can be
/// received without blocking. For a standard socket, this means
/// that at least one byte of data may be read from the fd without
/// blocking
pub const READABLE: Trigger = Trigger::READABLE;
/// Trigger an `Event` on write readiness.
///
/// For a `libzmq` socket this means that at least one message can
/// be sent without blocking. For a standard socket, this means
/// that at lesat one bye of data may be written to the fd without
/// blocking.
pub const WRITABLE: Trigger = Trigger::WRITABLE;

bitflags! {
    struct Cause: c_short {
        const READABLE = sys::ZMQ_POLLIN as c_short;
        const WRITABLE = sys::ZMQ_POLLOUT as c_short;
        const ERROR = sys::ZMQ_POLLERR as c_short;
        const PRIORITY = sys::ZMQ_POLLPRI as c_short;
    }
}

/// An alias to a [`Pollable`] element.
///
/// [`Pollable`]: enum.Pollable.html
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PollId(pub usize);

impl From<usize> for PollId {
    fn from(val: usize) -> PollId {
        PollId(val)
    }
}

impl From<PollId> for usize {
    fn from(val: PollId) -> usize {
        val.0
    }
}

/// A type that can be polled by the [`Poller`].
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{Server, poll::*};
/// use std::{net::TcpListener, os::unix::io::AsRawFd};
///
/// let mut poller = Poller::new();
///
/// // The poller can poll sockets using their handle...
/// let server = Server::new()?;
/// poller.add(server.handle(), PollId(0), READABLE)?;
///
/// // ...as well as a `RawFd`
/// let tcp_listener = TcpListener::bind("127.0.0.1:0")?;
/// poller.add(tcp_listener.as_raw_fd(), PollId(1), READABLE | WRITABLE)?;
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Poller`]: struct.Poller.html#method.add
pub trait Pollable {
    /// Add the pollable type to the poller.
    fn add(self, poller: &mut Poller, id: PollId, trigger: Trigger) -> Result<(), Error>;

    /// Remove the pollable type from the poller.
    fn remove(self, poller: &mut Poller) -> Result<(), Error>;

    /// Remove the pollable type from the poller.
    fn modify(self, poller: &mut Poller, trigger: Trigger) -> Result<(), Error>;
}

impl Pollable for SocketHandle {
    fn add(self, poller: &mut Poller, id: PollId, trigger: Trigger) -> Result<(), Error> {
        poller.add_socket(self, id, trigger)
    }

    fn remove(self, poller: &mut Poller) -> Result<(), Error> {
        poller.remove_socket(self)
    }

    fn modify(self, poller: &mut Poller, trigger: Trigger) -> Result<(), Error> {
        poller.modify_socket(self, trigger)
    }
}

impl Pollable for RawFd {
    fn add(self, poller: &mut Poller, id: PollId, trigger: Trigger) -> Result<(), Error> {
        poller.add_fd(self, id, trigger)
    }

    fn remove(self, poller: &mut Poller) -> Result<(), Error> {
        poller.remove_fd(self)
    }

    fn modify(self, poller: &mut Poller, trigger: Trigger) -> Result<(), Error> {
        poller.modify_fd(self, trigger)
    }
}

/// An `Iterator` over references to [`Event`].
///
/// Note that every event is guaranteed to be non-[`EMPTY`].
///
/// [`EMPTY`]: constant.EMPTY.html
/// [`Event`]: struct.Event.html
#[derive(Clone, Debug)]
pub struct Iter<'a> {
    inner: &'a Events,
    pos: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.inner.inner.get(self.pos);
        self.pos += 1;

        if let Some(event) = raw {
            // Skip empty events.
            if event.events == 0 {
                return self.next();
            } else {
                let user_data = event.user_data as *mut usize as usize;
                Some(Event {
                    id: PollId(user_data),
                    cause: Cause::from_bits(event.events).unwrap(),
                })
            }
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.inner.len();
        (len, Some(len))
    }
}

/// An `Iterator` over a set of [`Event`].
///
/// Note that every event is guaranteed to be non-[`EMPTY`].
///
/// [`EMPTY`]: constant.EMPTY.html
/// [`Event`]: struct.Event.html
#[derive(Debug)]
pub struct IntoIter {
    inner: Events,
    pos: usize,
}

impl Iterator for IntoIter {
    type Item = Event;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let raw = self.inner.inner.get(self.pos);
        self.pos += 1;

        raw.map(|raw| {
            let user_data = raw.user_data as *mut usize as usize;
            Event {
                id: PollId(user_data),
                cause: Cause::from_bits(raw.events).unwrap(),
            }
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.inner.len();
        (len, Some(len))
    }
}
/// An event detected by a poller.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Event {
    cause: Cause,
    id: PollId,
}

impl Event {
    /// Indicates read readiness.
    pub fn is_readable(&self) -> bool {
        self.cause.contains(Cause::READABLE)
    }

    /// Indicates write readiness.
    pub fn is_writable(&self) -> bool {
        self.cause.contains(Cause::WRITABLE)
    }

    /// Indicates that an error was received.
    ///
    /// This event can only occur if a `RawFd` is polled.
    pub fn is_error(&self) -> bool {
        self.cause.contains(Cause::ERROR)
    }

    /// Indicates that there is urgent data to be read.
    ///
    /// This event can only occur if a `RawFd` is polled.
    pub fn is_priority(&self) -> bool {
        self.cause.contains(Cause::PRIORITY)
    }

    /// Returns the [`PollId`] associated with the event.
    ///
    /// [`PollId`]: struct.PollId.html
    pub fn id(&self) -> PollId {
        self.id
    }
}

/// A vector of [`Event`] detected while polling.
///
/// [`Event`]: struct.Event.html
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Events {
    inner: Vec<sys::zmq_poller_event_t>,
}

impl Events {
    /// Creates a new empty event vector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new event vector with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    /// Returns the capacity of the event vector.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Returns `true` is the event vector contains no events.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the `Event` in the event vector.
    pub fn iter(&self) -> Iter {
        Iter {
            inner: &self,
            pos: 0,
        }
    }

    /// Empties the event vector.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<'a> IntoIterator for &'a Events {
    type Item = Event;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for Events {
    type Item = Event;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self,
            pos: 0,
        }
    }
}

/// A mechanism for input/output events multiplexing in a level-triggered fashion.
///
/// The poller is used to asynchronously detect events of a set of monitored
/// [`Pollable`] elements. These elements must be registered for monitoring by
/// [`adding`] them to the `Poller`.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *, poll::*};
///
/// // We initialize our sockets and connect them to each other.
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let server = Server::new()?;
/// server.bind(addr)?;
///
/// let bound = server.last_endpoint()?;
///
/// let client = Client::new()?;
/// client.connect(&bound)?;
///
/// // We create our poller instance.
/// let mut poller = Poller::new();
/// poller.add(server.handle(), PollId(0), READABLE)?;
/// poller.add(client.handle(), PollId(1), READABLE)?;
///
/// // Initialize the client.
/// client.send("ping")?;
///
/// let mut events = Events::new();
///
/// // Now the client and each server will send messages back and forth.
/// for _ in 0..100 {
///     // Wait indefinitely until at least one event is detected.
///     poller.poll(&mut events, Period::Infinite)?;
///     // Iterate over the detected events. Note that, since the events are
///     // guaranteed to be non-empty, we don't need to protect against spurious
///     // wakeups.
///     for event in &events {
///         assert!(event.is_readable());
///         match event.id() {
///             // The server is ready to receive an incoming message.
///             PollId(0) => {
///                 let msg = server.recv_msg()?;
///                 assert_eq!("ping", msg.to_str()?);
///                 server.send(msg)?;
///             }
///             // One of the clients is ready to receive an incoming message.
///             PollId(1) => {
///                 let msg = client.recv_msg()?;
///                 assert_eq!("ping", msg.to_str()?);
///                 client.send(msg)?;
///             }
///             _ => unreachable!(),
///         }
///     }
/// }
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Pollable`]: enum.Pollable.html
/// [`adding`]: #method.add
#[derive(Eq, PartialEq, Debug)]
pub struct Poller {
    poller: *mut c_void,
    count: usize,
}

impl Poller {
    /// Create a new empty poller.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a [`Pollable`] element for monitoring by the poller with the
    /// specified [`Trigger`] condition.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (added socket twice or invalid fd)
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{Server, poll::*, ErrorKind};
    ///
    /// let server = Server::new()?;
    ///
    /// let mut poller = Poller::new();
    ///
    /// poller.add(server.handle(), PollId(0), EMPTY)?;
    /// let err = poller.add(server.handle(), PollId(1), EMPTY).unwrap_err();
    ///
    /// match err.kind() {
    ///     ErrorKind::InvalidInput { .. } => (),
    ///     _ => panic!("unexpected error"),
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`Pollable`]: enum.Pollable.html
    /// [`Trigger`]: struct.Trigger.html
    pub fn add<P>(
        &mut self,
        pollable: P,
        id: PollId,
        trigger: Trigger,
    ) -> Result<(), Error>
    where
        P: Pollable,
    {
        pollable.add(self, id, trigger)
    }

    pub(crate) fn add_fd(
        &mut self,
        fd: RawFd,
        id: PollId,
        trigger: Trigger,
    ) -> Result<(), Error> {
        let user_data: usize = id.into();
        let user_data = user_data as *mut usize as *mut c_void;

        let rc = unsafe {
            sys::zmq_poller_add_fd(self.poller, fd, user_data, trigger.bits())
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = match errno {
                errno::EINVAL => {
                    Error::new(ErrorKind::InvalidInput("cannot add fd twice"))
                }
                errno::EBADF => Error::new(ErrorKind::InvalidInput(
                    "specified fd was the retired fd",
                )),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            self.count += 1;
            Ok(())
        }
    }

    pub(crate) fn add_socket(
        &mut self,
        handle: SocketHandle,
        id: PollId,
        trigger: Trigger,
    ) -> Result<(), Error> {
        let socket_ptr = handle.as_ptr();

        let user_data: usize = id.into();
        let user_data = user_data as *mut usize as *mut c_void;

        let rc = unsafe {
            sys::zmq_poller_add(
                self.poller,
                socket_ptr,
                user_data,
                trigger.bits(),
            )
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                    "cannot add socket twice",
                )),
                errno::ENOTSOCK => Error::new(ErrorKind::InvalidSocket),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            self.count += 1;
            Ok(())
        }
    }

    /// Remove a [`Pollable`] element from monitoring by the poller.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (element not present or invalid fd)
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{Server, poll::*, ErrorKind};
    ///
    /// let server = Server::new()?;
    /// let handle = server.handle();
    /// let mut poller = Poller::new();
    ///
    /// poller.add(handle, PollId(0), EMPTY)?;
    /// poller.remove(handle)?;
    ///
    /// let err = poller.remove(handle).unwrap_err();
    /// match err.kind() {
    ///     ErrorKind::InvalidInput { .. } => (), // cannot remove socket twice.
    ///     _ => panic!("unexpected error"),
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`Pollable`]: enum.Pollable.html
    pub fn remove<'a, P>(&mut self, pollable: P) -> Result<(), Error>
    where
        P: Pollable,
    {
        pollable.remove(self)
    }

    pub(crate) fn remove_fd(&mut self, fd: RawFd) -> Result<(), Error> {
        let rc = unsafe { sys::zmq_poller_remove_fd(self.poller, fd) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                    "cannot remove absent fd",
                )),
                errno::EBADF => Error::new(ErrorKind::InvalidInput(
                    "specified fd was the retired fd",
                )),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            self.count -= 1;
            Ok(())
        }
    }

    pub(crate) fn remove_socket(
        &mut self,
        handle: SocketHandle,
    ) -> Result<(), Error> {
        let socket_ptr = handle.as_ptr();

        let rc = unsafe { sys::zmq_poller_remove(self.poller, socket_ptr) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = match errno {
                errno::ENOTSOCK => Error::new(ErrorKind::InvalidSocket),
                errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                    "cannot remove absent socket",
                )),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            self.count -= 1;
            Ok(())
        }
    }

    /// Modify the [`Trigger`] confition for the specified [`Pollable`] element
    /// monitored by the poller.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (element not present or invalid fd)
    ///
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`Pollable`]: enum.Pollable.html
    /// [`Trigger`]: struct.Trigger.html
    pub fn modify<'a, P>(
        &mut self,
        pollable: P,
        trigger: Trigger,
    ) -> Result<(), Error>
    where
        P: Pollable,
    {
        pollable.modify(self, trigger)
    }

    pub(crate) fn modify_fd(&mut self, fd: RawFd, trigger: Trigger) -> Result<(), Error> {
        let rc = unsafe {
            sys::zmq_poller_modify_fd(self.poller, fd, trigger.bits())
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                    "cannot modify absent fd",
                )),
                errno::EBADF => Error::new(ErrorKind::InvalidInput(
                    "specified fd is the retired fd",
                )),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            Ok(())
        }
    }

    pub(crate) fn modify_socket(
        &mut self,
        handle: SocketHandle,
        trigger: Trigger,
    ) -> Result<(), Error> {
        let socket_ptr = handle.as_ptr();

        let rc = unsafe {
            sys::zmq_poller_modify(self.poller, socket_ptr, trigger.bits())
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                    "cannot modify absent socket",
                )),
                errno::ENOTSOCK => Error::new(ErrorKind::InvalidSocket),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            Ok(())
        }
    }

    fn wait(&mut self, events: &mut Events, timeout: i64) -> Result<(), Error> {
        events.clear();
        for _i in 0..self.count {
            events.inner.push(sys::zmq_poller_event_t::default());
        }
        let rc = unsafe {
            sys::zmq_poller_wait_all(
                self.poller,
                events.inner.as_mut_ptr(),
                events.inner.len() as i32,
                timeout,
            )
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = match errno {
                errno::EINVAL => panic!("invalid poller"),
                errno::ETERM => Error::new(ErrorKind::InvalidCtx),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
                _ => panic!(msg_from_errno(errno)),
            };

            Err(err)
        } else {
            Ok(())
        }
    }

    /// Check for events in the monitored elements, returning instantly.
    ///
    /// If no events occured, returns [`WouldBlock`]. Note that in this case,
    /// the `Events` would also be empty.
    ///
    /// # Returned Errors
    /// * [`WouldBlock`]
    /// * [`InvalidCtx`] (`Ctx` of a polled socket was terminated)
    /// * [`Interrupted`]
    ///
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.Interrupted
    pub fn try_poll(&mut self, events: &mut Events) -> Result<(), Error> {
        self.wait(events, 0)
    }

    /// The poller will wait for events in the monitored elements,
    /// blocking until at least one event occurs, or the specified
    /// timeout `Period` expires.
    ///
    /// If the specified `Period` is infinite, the poller will block forever
    /// until an event occurs.
    ///
    /// # Returned Errors
    /// * [`WouldBlock`] (timeout expired)
    /// * [`InvalidCtx`] (`Ctx` of a polled socket was terminated)
    /// * [`Interrupted`]
    ///
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    pub fn poll(
        &mut self,
        events: &mut Events,
        timeout: Period,
    ) -> Result<(), Error> {
        match timeout {
            Period::Finite(duration) => {
                let ms = duration.as_millis();
                if ms > i64::max_value() as u128 {
                    return Err(Error::new(ErrorKind::InvalidInput(
                        "ms in timeout must be less than i64::MAX",
                    )));
                }
                self.wait(events, ms as i64)
            }
            Period::Infinite => self.wait(events, -1),
        }
    }
}

impl Default for Poller {
    fn default() -> Self {
        let poller = unsafe { sys::zmq_poller_new() };

        if poller.is_null() {
            panic!(msg_from_errno(unsafe { sys::zmq_errno() }));
        }

        Self { poller, count: 0 }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        let rc = unsafe { sys::zmq_poller_destroy(&mut self.poller) };

        if rc != 0 {
            let errno = unsafe { sys::zmq_errno() };

            match errno {
                errno::EFAULT => panic!("invalid poller"),
                _ => panic!(msg_from_errno(errno)),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_trigger() {
        assert_eq!(READABLE.bits(), sys::ZMQ_POLLIN as c_short);
        assert_eq!(WRITABLE.bits(), sys::ZMQ_POLLOUT as c_short);
    }
}
