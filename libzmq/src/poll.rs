//! Asynchronous polling mechanim.
//!
//! See the [`Poller`] documentation to get started.
//!
//! [`Poller`]: struct.Poller.html

use crate::{
    core::{GetRawSocket, Period, RawSocket},
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
    /// A bitflag used to specifies the condition for triggering an event in the
    /// [`Poller`].
    ///
    /// # Example
    /// ```
    /// use libzmq::poll::*;
    ///
    /// // This specifies to the poller to trigger an event if `Pollable`
    /// // is readable, writable, or both.
    /// let either = READABLE | WRITABLE;
    /// ```
    ///
    /// [`Poller`]: struct.Poller.html
    pub struct Trigger: c_short {
        /// Never trigger.
        const EMPTY = 0b00_000_000;
        /// Trigger an `Event` on read readiness.
        const READABLE = 0b00_000_001;
        /// Trigger an `Event` on write readiness.
        const WRITABLE = 0b00_000_010;
    }
}

/// Never trigger.
pub const EMPTY: Trigger = Trigger::EMPTY;
/// Trigger an `Event` on read readiness.
pub const READABLE: Trigger = Trigger::READABLE;
/// Trigger an `Event` on write readiness.
pub const WRITABLE: Trigger = Trigger::WRITABLE;

/// An alias to a [`Pollable`] element.
///
/// [`Pollable`]: enum.Pollable.html
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub usize);

impl From<usize> for Id {
    fn from(val: usize) -> Id {
        Id(val)
    }
}

impl From<Id> for usize {
    fn from(val: Id) -> usize {
        val.0
    }
}

/// The types that can be polled by the [`Poller`].
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{Server, poll::*};
/// use std::net::TcpListener;
///
/// let mut poller = Poller::new();
///
/// // The poller can poll sockets...
/// let server = Server::new()?;
/// poller.add(&server, Id(0), READABLE)?;
///
/// // ...as well as any type that implements `AsRawFd`.
/// let tcp_listener = TcpListener::bind("127.0.0.1:0")?;
/// poller.add(&tcp_listener, Id(1), READABLE | WRITABLE)?;
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Poller`]: struct.Poller.html#method.add
pub enum Pollable<'a> {
    /// A `Socket` type.
    Socket(&'a RawSocket),
    /// A file descriptor.
    Fd(RawFd),
}

impl<'a, T> From<&'a T> for Pollable<'a>
where
    T: AsRawFd,
{
    fn from(entity: &'a T) -> Self {
        Pollable::Fd(entity.as_raw_fd())
    }
}

impl<'a> From<&'a Client> for Pollable<'a> {
    fn from(client: &'a Client) -> Self {
        Pollable::Socket(client.raw_socket())
    }
}

impl<'a> From<&'a Server> for Pollable<'a> {
    fn from(server: &'a Server) -> Self {
        Pollable::Socket(server.raw_socket())
    }
}

impl<'a> From<&'a Radio> for Pollable<'a> {
    fn from(radio: &'a Radio) -> Self {
        Pollable::Socket(radio.raw_socket())
    }
}

impl<'a> From<&'a Dish> for Pollable<'a> {
    fn from(dish: &'a Dish) -> Self {
        Pollable::Socket(dish.raw_socket())
    }
}

impl<'a> From<&'a Gather> for Pollable<'a> {
    fn from(gather: &'a Gather) -> Self {
        Pollable::Socket(gather.raw_socket())
    }
}

impl<'a> From<&'a Scatter> for Pollable<'a> {
    fn from(scatter: &'a Scatter) -> Self {
        Pollable::Socket(scatter.raw_socket())
    }
}

#[doc(hidden)]
impl<'a> From<&'a OldSocket> for Pollable<'a> {
    fn from(old: &'a OldSocket) -> Self {
        Pollable::Socket(old.raw_socket())
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
                    id: Id(user_data),
                    trigger: Trigger::from_bits(event.events).unwrap(),
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
                id: Id(user_data),
                trigger: Trigger::from_bits(raw.events).unwrap(),
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
    trigger: Trigger,
    id: Id,
}

impl Event {
    /// Returns the condition that triggered the event.
    ///
    /// This is guarantee to never be [`EMPTY`].
    ///
    /// [`EMPTY`]: constant.EMPTY.html
    pub fn trigger(&self) -> Trigger {
        self.trigger
    }

    /// Returns the [`Id`] associated with the event.
    ///
    /// [`Id`]: struct.Id.html
    pub fn id(&self) -> Id {
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
/// poller.add(&server, Id(0), READABLE)?;
/// poller.add(&client, Id(1), READABLE)?;
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
///         match event.id() {
///             // The server is ready to receive an incoming message.
///             Id(0) => {
///                 let msg = server.recv_msg()?;
///                 assert_eq!("ping", msg.to_str()?);
///                 server.send(msg)?;
///             }
///             // One of the clients is ready to receive an incoming message.
///             Id(1) => {
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
    /// poller.add(&server, Id(0), EMPTY)?;
    /// let err = poller.add(&server, Id(1), EMPTY).unwrap_err();
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
    pub fn add<'a, P>(
        &mut self,
        pollable: P,
        id: Id,
        trigger: Trigger,
    ) -> Result<(), Error>
    where
        P: Into<Pollable<'a>>,
    {
        match pollable.into() {
            Pollable::Socket(raw_socket) => {
                self.add_raw_socket(raw_socket, id, trigger)
            }
            Pollable::Fd(fd) => self.add_fd(fd, id, trigger),
        }
    }

    fn add_fd(
        &mut self,
        fd: RawFd,
        id: Id,
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

    fn add_raw_socket(
        &mut self,
        raw_socket: &RawSocket,
        id: Id,
        trigger: Trigger,
    ) -> Result<(), Error> {
        let socket_mut_ptr = raw_socket.as_mut_ptr();

        let user_data: usize = id.into();
        let user_data = user_data as *mut usize as *mut c_void;

        let rc = unsafe {
            sys::zmq_poller_add(
                self.poller,
                socket_mut_ptr,
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
                errno::ENOTSOCK => panic!("invalid socket"),
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
    /// let mut poller = Poller::new();
    ///
    /// poller.add(&server, Id(0), EMPTY)?;
    /// poller.remove(&server)?;
    ///
    /// let err = poller.remove(&server).unwrap_err();
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
        P: Into<Pollable<'a>>,
    {
        match pollable.into() {
            Pollable::Socket(raw_socket) => self.remove_raw_socket(raw_socket),
            Pollable::Fd(fd) => self.remove_fd(fd),
        }
    }

    fn remove_fd(&mut self, fd: RawFd) -> Result<(), Error> {
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

    fn remove_raw_socket(
        &mut self,
        raw_socket: &RawSocket,
    ) -> Result<(), Error> {
        let socket_mut_ptr = raw_socket.as_mut_ptr();

        let rc = unsafe { sys::zmq_poller_remove(self.poller, socket_mut_ptr) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = match errno {
                errno::ENOTSOCK => panic!("invalid socket"),
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
        P: Into<Pollable<'a>>,
    {
        match pollable.into() {
            Pollable::Socket(raw_socket) => {
                self.modify_raw_socket(raw_socket, trigger)
            }
            Pollable::Fd(fd) => self.modify_fd(fd, trigger),
        }
    }

    fn modify_fd(&mut self, fd: RawFd, trigger: Trigger) -> Result<(), Error> {
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

    fn modify_raw_socket(
        &mut self,
        raw_socket: &RawSocket,
        trigger: Trigger,
    ) -> Result<(), Error> {
        let socket_mut_ptr = raw_socket.as_mut_ptr();

        let rc = unsafe {
            sys::zmq_poller_modify(self.poller, socket_mut_ptr, trigger.bits())
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = match errno {
                errno::EINVAL => Error::new(ErrorKind::InvalidInput(
                    "cannot modify absent socket",
                )),
                errno::ENOTSOCK => panic!("invalid socket"),
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
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
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
    /// * [`CtxTerminated`] (`Ctx` of a polled socket was terminated)
    /// * [`Interrupted`]
    ///
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
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
    /// * [`CtxTerminated`] (`Ctx` of a polled socket was terminated)
    /// * [`Interrupted`]
    ///
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
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
