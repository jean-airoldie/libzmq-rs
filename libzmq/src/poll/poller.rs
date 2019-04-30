use crate::{
    core::GetRawSocket,
    error::{msg_from_errno, Error, ErrorKind},
};

use libzmq_sys as sys;
use sys::errno;

use bitflags::bitflags;

use hashbrown::HashMap;

use std::{
    os::raw::{c_short, c_void},
    time::Duration,
    vec,
};

bitflags! {
    /// The event flags that can be specified to the poller.
    pub struct PollFlags: c_short {
        /// Specifies no wakeup condition at all.
        const NO_WAKEUP = 0b00_000_000;
        /// Specifies wakeup on read readiness event.
        const READABLE = 0b00_000_001;
        /// Specifies wakeup on write readiness event.
        const WRITABLE = 0b00_000_010;
    }
}

/// Specifies no wakeup condition at all.
pub const NO_WAKEUP: PollFlags = PollFlags::NO_WAKEUP;
/// Specifies wakeup on read readiness.
pub const READABLE: PollFlags = PollFlags::READABLE;
/// Specifies wakeup on write readiness.
pub const WRITABLE: PollFlags = PollFlags::WRITABLE;

/// An iterator over a set of [`PollEvent`].
///
/// [`PollEvent`]: struct.PollEvent.html
#[derive(Clone, Debug)]
pub struct PollIter<'a, T> {
    inner: vec::IntoIter<PollEvent<'a, T>>,
}

impl<'a, T> Iterator for PollIter<'a, T> {
    type Item = PollEvent<'a, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An event detected by a poller.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PollEvent<'a, T> {
    flags: PollFlags,
    user_data: &'a T,
}

impl<'a, T> PollEvent<'a, T> {
    /// Specifies the kind of event that was triggered.
    ///
    /// It will never be equal to [`NO_WAKEUP`].
    ///
    /// [`NO_WAKEUP`]: constant.NO_WAKEUP.html
    pub fn flags(&self) -> PollFlags {
        self.flags
    }

    /// Returns a reference to the user data that was provided when
    /// calling [`add`].
    ///
    /// [`add`]: struct.Poller.html#method.add
    pub fn user_data(&self) -> &'a T {
        self.user_data
    }
}

/// A mechanism for input/output events multiplexing in a level-triggered fashion.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{*, prelude::*};
///
/// // This is the arbitrary user data that we pass to the poller.
/// // Here we pass a reference to a socket which we will use in the loop.
/// enum Which<'a> {
///     Server(&'a Server),
///     Client(&'a Client),
/// };
///
/// // We initialize our sockets and connect them to each other.
/// const ENDPOINT: &str = "inproc://test";
///
/// let server = Server::new()?;
/// server.bind(ENDPOINT)?;
///
/// // We create an arbitrary number of clients.
/// let clients = {
///     let mut vec = Vec::with_capacity(3);
///     for _ in 0..3 {
///         let client = Client::new()?;
///         client.connect(ENDPOINT)?;
///         vec.push(client);
///     }
///     vec
/// };
///
/// // We create our poller instance.
/// let mut poller = Poller::new();
/// // In this example we will solely poll for incoming messages.
/// poller.add(&server, Which::Server(&server), READABLE)?;
/// for client in &clients {
///     poller.add(client, Which::Client(client), READABLE)?;
/// }
///
/// // We send the initial request for each client.
/// for client in &clients {
///     client.send("ping")?;
/// }
///
/// // Now the client and each server will send messages back and forth.
/// for _ in 0..100 {
///     // This waits indefinitely until at least one event is detected. Since many
///     // events can be detected at once, it returns an iterator.
///     for event in poller.block(None)? {
///         assert_eq!(READABLE, event.flags());
///         // Note that `user_data` is the `Which` that we
///         // passed in the `Poller::add` method.
///         match event.user_data() {
///             // The server is ready to receive an incoming message.
///             Which::Server(server) => {
///                 let msg = server.recv_msg()?;
///                 assert_eq!("ping", msg.to_str()?);
///                 server.send(msg)?;
///             }
///             // One of the clients is ready to receive an incoming message.
///             Which::Client(client) => {
///                 let msg = client.recv_msg()?;
///                 assert_eq!("ping", msg.to_str()?);
///                 client.send(msg)?;
///             }
///         }
///     }
/// }
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Eq, PartialEq, Debug)]
pub struct Poller<T> {
    poller: *mut c_void,
    raw_event_vec: Vec<sys::zmq_poller_event_t>,
    user_data: Vec<T>,
    free_slots: Vec<usize>,
    used_slots: HashMap<*mut c_void, usize>,
}

impl<T> Poller<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{Server, poll::*, error::ErrorKind};
    ///
    /// let server = Server::new()?;
    ///
    /// let mut poller = Poller::new();
    ///
    /// poller.add(&server, 0, NO_WAKEUP)?;
    /// let err = poller.add(&server, 0, NO_WAKEUP).unwrap_err();
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
        user_data: T,
        flags: PollFlags,
    ) -> Result<(), Error<()>> {
        // This is safe since we won't actually mutate the socket.
        let mut_raw_socket = socket.raw_socket() as *mut c_void;

        if self.used_slots.get(&mut_raw_socket).is_some() {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "socket already added",
            }));
        }

        let slot = {
            match self.free_slots.pop() {
                Some(slot) => {
                    self.user_data[slot] = user_data;
                    slot
                }
                None => {
                    self.user_data.push(user_data);
                    self.user_data.len() - 1
                }
            }
        };

        let mut_user_data = slot as *mut c_void;

        let rc = unsafe {
            sys::zmq_poller_add(
                self.poller,
                mut_raw_socket,
                mut_user_data,
                flags.bits(),
            )
        };

        if rc == -1 {
            self.free_slots.push(slot);

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
            self.used_slots.insert(mut_raw_socket, slot);
            self.raw_event_vec.push(sys::zmq_poller_event_t::default());

            Ok(())
        }
    }

    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{poll::*, Server, error::ErrorKind};
    ///
    /// let server = Server::new()?;
    /// let mut poller = Poller::new();
    ///
    /// poller.add(&server, 0, NO_WAKEUP)?;
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
    pub fn remove(&mut self, socket: &GetRawSocket) -> Result<(), Error<()>> {
        // This is safe since we don't actually mutate the socket.
        let mut_raw_socket = socket.raw_socket() as *mut c_void;

        if self.used_slots.get(&mut_raw_socket).is_none() {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "cannot remove absent socket",
            }));
        }

        let rc = unsafe { sys::zmq_poller_remove(self.poller, mut_raw_socket) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            match errno {
                errno::ENOTSOCK => panic!("invalid socket"),
                _ => panic!(msg_from_errno(errno)),
            }
        } else {
            let slot = self.used_slots.remove(&mut_raw_socket).unwrap();
            self.free_slots.push(slot);

            Ok(())
        }
    }

    pub fn modify<S>(
        &mut self,
        socket: &GetRawSocket,
        flags: PollFlags,
    ) -> Result<(), Error<()>> {
        // This is safe since we don't actually mutate the socket.
        let mut_raw_socket = socket.raw_socket() as *mut c_void;

        if self.used_slots.get(&mut_raw_socket).is_some() {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "cannot modify absent socket",
            }));
        }

        let rc = unsafe {
            sys::zmq_poller_modify(self.poller, mut_raw_socket, flags.bits())
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

    fn wait(&mut self, timeout: i64) -> Result<PollIter<T>, Error<()>> {
        let len = self.raw_event_vec.len();

        let rc = unsafe {
            sys::zmq_poller_wait_all(
                self.poller,
                self.raw_event_vec.as_mut_ptr(),
                len as i32,
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
            let mut polled = Vec::with_capacity(rc as usize);

            for i in 0..rc as usize {
                let event = self.raw_event_vec[i];
                let flags = PollFlags::from_bits(event.events).unwrap();
                let slot = event.user_data as usize;
                let user_data = &self.user_data[slot];
                polled.push(PollEvent { flags, user_data });
            }

            Ok(PollIter {
                inner: polled.into_iter(),
            })
        }
    }

    /// The poller will poll for events, returning instantly.
    ///
    /// If there are none, returns [`WouldBlock`].
    pub fn poll(&mut self) -> Result<PollIter<T>, Error<()>> {
        self.wait(0)
    }

    /// The poller will block until at least an event occurs.
    ///
    /// If a duration is specified, the poller will wait for at most the
    /// duration for an event before it returns [`WouldBlock`].
    pub fn block(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<PollIter<T>, Error<()>> {
        match timeout {
            Some(duration) => {
                let ms = duration.as_millis();
                if ms > i64::max_value() as u128 {
                    return Err(Error::new(ErrorKind::InvalidInput {
                        msg: "ms in timeout must be less than i64::MAX",
                    }));
                }
                self.wait(ms as i64)
            }
            None => self.wait(-1),
        }
    }
}

impl<T> Default for Poller<T> {
    fn default() -> Self {
        let poller = unsafe { sys::zmq_poller_new() };

        if poller.is_null() {
            panic!(msg_from_errno(unsafe { sys::zmq_errno() }));
        }

        Self {
            poller,
            raw_event_vec: vec![],
            user_data: Vec::new(),
            free_slots: Vec::new(),
            used_slots: HashMap::new(),
        }
    }
}

impl<T> Drop for Poller<T> {
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
        assert_eq!(PollFlags::READABLE.bits(), sys::ZMQ_POLLIN as c_short);
        assert_eq!(PollFlags::WRITABLE.bits(), sys::ZMQ_POLLOUT as c_short);
    }

    #[test]
    fn test_remove_absent_socket() {
        use crate::Server;

        let server = Server::new().unwrap();

        let mut poller = Poller::<i32>::new();
        let err = poller.remove(&server).unwrap_err();

        match err.kind() {
            ErrorKind::InvalidInput { .. } => (),
            _ => panic!("unexpected error"),
        }
    }

    #[test]
    fn test_poller() {
        use crate::{prelude::*, Client, Server};

        // This is the arbitrary user data that we pass to the poller.
        // Here we pass a reference to a socket which we will use in the loop.
        enum Which<'a> {
            Server(&'a Server),
            Client(&'a Client),
        };

        // We initialize our sockets and connect them to each other.
        const ENDPOINT: &str = "inproc://test";

        let server = Server::new().unwrap();
        server.bind(ENDPOINT).unwrap();

        // We create an arbitrary number of clients.
        let clients = {
            let mut vec = Vec::with_capacity(3);
            for _ in 0..3 {
                let client = Client::new().unwrap();
                client.connect(ENDPOINT).unwrap();
                vec.push(client);
            }
            vec
        };

        // We create our poller instance.
        let mut poller = Poller::new();
        // In this example we will solely poll for incoming messages.
        poller
            .add(&server, Which::Server(&server), READABLE)
            .unwrap();
        for client in &clients {
            poller.add(client, Which::Client(client), READABLE).unwrap();
        }

        // We send the initial request for each client.
        for client in &clients {
            client.send("ping").unwrap();
        }

        // Now the client and each server will send messages back and forth.
        for _ in 0..100 {
            // This waits indefinitely until at least one event is detected. Since many
            // events can be detected at once, it returns an iterator.
            for event in poller.block(None).unwrap() {
                assert_eq!(READABLE, event.flags());
                // Note that `user_data` is the `Which` that we
                // passed in the `Poller::add` method.
                match event.user_data() {
                    // The server is ready to receive an incoming message.
                    Which::Server(server) => {
                        let msg = server.recv_msg().unwrap();
                        assert_eq!("ping", msg.to_str().unwrap());
                        server.send(msg).unwrap();
                    }
                    // One of the clients is ready to receive an incoming message.
                    Which::Client(client) => {
                        let msg = client.recv_msg().unwrap();
                        assert_eq!("ping", msg.to_str().unwrap());
                        client.send(msg).unwrap();
                    }
                }
            }
        }
    }
}
