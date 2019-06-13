//! Asynchronous polling mechanim.

use crate::{
    core::{GetRawSocket, Period},
    error::{msg_from_errno, Error, ErrorKind},
};

use libzmq_sys as sys;
use sys::errno;

use bitflags::bitflags;

use std::os::raw::{c_short, c_void};

bitflags! {
    /// The event flags that can be specified to the poller.
    pub struct Flags: c_short {
        /// Specifies no wakeup condition at all.
        const NO_WAKEUP = 0b00_000_000;
        /// Specifies wakeup on read readiness event.
        const READABLE = 0b00_000_001;
        /// Specifies wakeup on write readiness event.
        const WRITABLE = 0b00_000_010;
    }
}

/// Specifies no wakeup condition at all.
pub const NO_WAKEUP: Flags = Flags::NO_WAKEUP;
/// Specifies wakeup on read readiness.
pub const READABLE: Flags = Flags::READABLE;
/// Specifies wakeup on write readiness.
pub const WRITABLE: Flags = Flags::WRITABLE;

/// The type used to alias a socket or a `RawFd` when polling.
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

/// An `Iterator` over references to [`Event`].
///
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
                    flags: Flags::from_bits(event.events).unwrap(),
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
                flags: Flags::from_bits(raw.events).unwrap(),
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
    flags: Flags,
    id: PollId,
}

impl Event {
    /// Specifies the kind of event that was triggered.
    ///
    /// It will never be equal to [`NO_WAKEUP`].
    ///
    /// [`NO_WAKEUP`]: constant.NO_WAKEUP.html
    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn id(&self) -> PollId {
        self.id
    }
}

/// Used to store [`Event`]s for polling.
///
/// [`Event`]: struct.Event.html
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Events {
    inner: Vec<sys::zmq_poller_event_t>,
}

impl Events {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> Iter {
        Iter {
            inner: &self,
            pos: 0,
        }
    }

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
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, *, poll::*};
/// use std::convert::TryInto;
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
/// poller.add(&server, PollId(0), READABLE)?;
/// poller.add(&client, PollId(1), READABLE)?;
///
/// // Initialize the client.
/// client.send("ping")?;
///
/// let mut events = Events::new();
///
/// // Now the client and each server will send messages back and forth.
/// for _ in 0..100 {
///     // Wait indefinitely until at least one event is detected.
///     poller.block(&mut events, Period::Infinite)?;
///     // Iterate over the detected events.
///     for event in &events {
///         // Guard against spurious wakeups.
///         if event.flags() != NO_WAKEUP {
///             match event.id() {
///                 // The server is ready to receive an incoming message.
///                 PollId(0) => {
///                     let msg = server.recv_msg()?;
///                     assert_eq!("ping", msg.to_str()?);
///                     server.send(msg)?;
///                 }
///                 // One of the clients is ready to receive an incoming message.
///                 PollId(1) => {
///                     let msg = client.recv_msg()?;
///                     assert_eq!("ping", msg.to_str()?);
///                     client.send(msg)?;
///                 }
///                 _ => unreachable!(),
///             }
///         }
///     }
/// }
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Eq, PartialEq, Debug)]
pub struct Poller {
    poller: *mut c_void,
    count: usize,
}

impl Poller {
    pub fn new() -> Self {
        Self::default()
    }

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
    /// poller.add(&server, PollId(0), NO_WAKEUP)?;
    /// let err = poller.add(&server, PollId(1), NO_WAKEUP).unwrap_err();
    ///
    /// match err.kind() {
    ///     ErrorKind::InvalidInput { .. } => (),
    ///     _ => panic!("unexpected error"),
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn add(
        &mut self,
        socket: &GetRawSocket,
        id: PollId,
        flags: Flags,
    ) -> Result<(), Error> {
        let socket_mut_ptr = socket.raw_socket().as_mut_ptr();

        let user_data: usize = id.into();
        let user_data = user_data as *mut usize as *mut c_void;

        let rc = unsafe {
            sys::zmq_poller_add(
                self.poller,
                socket_mut_ptr,
                user_data,
                flags.bits(),
            )
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };
            let err = {
                match errno {
                    errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                        msg: "cannot add socket twice",
                    }),
                    errno::ENOTSOCK => panic!("invalid socket"),
                    _ => panic!(msg_from_errno(errno)),
                }
            };

            Err(err)
        } else {
            self.count += 1;
            Ok(())
        }
    }

    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{Server, poll::*, ErrorKind};
    ///
    /// let server = Server::new()?;
    /// let mut poller = Poller::new();
    ///
    /// poller.add(&server, PollId(0), NO_WAKEUP)?;
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
    pub fn remove(&mut self, socket: &GetRawSocket) -> Result<(), Error> {
        let socket_mut_ptr = socket.raw_socket().as_mut_ptr();

        let rc = unsafe { sys::zmq_poller_remove(self.poller, socket_mut_ptr) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = {
                match errno {
                    errno::ENOTSOCK => panic!("invalid socket"),
                    errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                        msg: "cannot remove absent socket",
                    }),
                    _ => panic!(msg_from_errno(errno)),
                }
            };

            Err(err)
        } else {
            self.count -= 1;
            Ok(())
        }
    }

    pub fn modify(
        &mut self,
        socket: &GetRawSocket,
        flags: Flags,
    ) -> Result<(), Error> {
        let socket_mut_ptr = socket.raw_socket().as_mut_ptr();

        let rc = unsafe {
            sys::zmq_poller_modify(self.poller, socket_mut_ptr, flags.bits())
        };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            match errno {
                errno::ENOTSOCK => panic!("invalid socket"),
                _ => panic!(msg_from_errno(errno)),
            }
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
            let err = {
                match errno {
                    errno::EINVAL => panic!("invalid poller"),
                    errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                    errno::EINTR => Error::new(ErrorKind::Interrupted),
                    errno::EAGAIN => Error::new(ErrorKind::WouldBlock),
                    _ => panic!(msg_from_errno(errno)),
                }
            };

            Err(err)
        } else {
            Ok(())
        }
    }

    /// The poller will poll for events, returning instantly.
    ///
    /// If there are none, returns [`WouldBlock`].
    pub fn poll(&mut self, events: &mut Events) -> Result<(), Error> {
        self.wait(events, 0)
    }

    /// The poller will block until at least an event occurs.
    ///
    /// If a duration is specified, the poller will wait for at most the
    /// duration for an event before it returns [`WouldBlock`].
    pub fn block(
        &mut self,
        events: &mut Events,
        timeout: Period,
    ) -> Result<(), Error> {
        match timeout {
            Period::Finite(duration) => {
                let ms = duration.as_millis();
                if ms > i64::max_value() as u128 {
                    return Err(Error::new(ErrorKind::InvalidInput {
                        msg: "ms in timeout must be less than i64::MAX",
                    }));
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
    fn test_flags() {
        assert_eq!(READABLE.bits(), sys::ZMQ_POLLIN as c_short);
        assert_eq!(WRITABLE.bits(), sys::ZMQ_POLLOUT as c_short);
    }
}
