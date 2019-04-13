use crate::{
    error::{msg_from_errno, Error, ErrorKind},
    socket::AsRawSocket,
};

use libzmq_sys as sys;
use sys::errno;

use bitflags::bitflags;

use hashbrown::HashMap;

use std::{
    fmt::Debug,
    marker::PhantomData,
    os::{
        raw::{c_short, c_void},
        unix::io::RawFd,
    },
    vec,
};

bitflags! {
    pub struct PollEvents: c_short {
        const NO_POLL = 0b00_000_000;
        const INCOMING = 0b00_000_001;
        const OUTGOING = 0b00_000_010;
    }
}

pub const NO_POLL: PollEvents = PollEvents::NO_POLL;
pub const INCOMING: PollEvents = PollEvents::INCOMING;
pub const OUTGOING: PollEvents = PollEvents::OUTGOING;

pub struct PollIter<'a, T> {
    inner: vec::IntoIter<PollItem<'a, T>>,
}

impl<'a, T> Iterator for PollIter<'a, T> {
    type Item = PollItem<'a, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[derive(Debug)]
pub struct PollItem<'a, T> {
    events: PollEvents,
    user_data: &'a T,
}

impl<'a, T> PollItem<'a, T> {
    pub fn events(&self) -> PollEvents {
        self.events
    }

    pub fn user_data(&self) -> &'a T {
        self.user_data
    }
}

#[doc(hidden)]
impl<'a, T> From<&'a sys::zmq_poller_event_t> for PollItem<'a, T>
where
    T: Debug,
{
    fn from(poll: &'a sys::zmq_poller_event_t) -> Self {
        let events = PollEvents::from_bits(poll.events).unwrap();
        //panic!("unsafe af");
        // This is safe since its a reference into the `Poller`, meaning it has
        // the right lifetime.
        let user_data = unsafe { &*(poll.user_data as *const T) };

        Self { events, user_data }
    }
}

///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
///
/// use libzmq::prelude::*;
///
/// // This is the arbitrary user data that we pass to the poller.
/// // Here we pass a reference to a socket which we will use in the loop.
/// #[derive(Debug)]
/// enum Which<'a> {
///     Server(&'a Server),
///     Client(&'a Client),
/// };
///
/// // We initialize our sockets and connect them to each other.
/// let endpoint: Endpoint = "inproc://test".parse()?;
///
/// let server = Server::new()?;
/// server.bind(&endpoint)?;
///
/// // We create an arbitrary number of clients.
/// let clients = {
///     let mut vec = Vec::with_capacity(3);
///     for _ in 0..3 {
///         let client = Client::new()?;
///         client.connect(&endpoint)?;
///         vec.push(client);
///     }
///     vec
/// };
///
/// // We create our poller instance.
/// let mut poller = Poller::new();
/// // In this example we will solely poll for incoming messages.
/// poller.add(&server, Which::Server(&server), INCOMING)?;
/// for client in &clients {
///     poller.add(client, Which::Client(client), INCOMING)?;
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
///     for poll in poller.wait(-1)? {
///         assert_eq!(INCOMING, poll.events());
///         // Note that `user_data` is a reference to our user provided `Which` type.
///         match poll.user_data() {
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
pub struct Poller<T> {
    poller: *mut c_void,
    vec: Vec<sys::zmq_poller_event_t>,
    map: HashMap<*mut c_void, T>,
    phantom: PhantomData<T>,
}

impl<T> Poller<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::prelude::*;
    ///
    /// let server = Server::new()?;
    ///
    /// let mut poller = Poller::new();
    ///
    /// poller.add(&server, 0, NO_POLL)?;
    /// let err = poller.add(&server, 0, NO_POLL).unwrap_err();
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
        socket: &AsRawSocket,
        user_data: T,
        events: PollEvents,
    ) -> Result<(), Error<()>>
    where
        T: Debug,
    {
        // This is safe since we won't actually mutate the socket.
        let mut_raw_socket = socket.as_raw_socket() as *mut _;

        if let Some(_) = self.map.get(&mut_raw_socket) {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "socket already added",
            }));
        }
        // This is sketchy since it means there will be a reference to our
        // map at all time. Thus we have to make sure not to invalidate this
        // reference.
        let user_data = self.map.entry(mut_raw_socket).or_insert(user_data);
        let mut_user_data_ptr = user_data as *const T as *mut _;

        let rc = unsafe {
            sys::zmq_poller_add(
                self.poller,
                mut_raw_socket,
                mut_user_data_ptr,
                events.bits(),
            )
        };

        if rc == -1 {
            self.map.remove(&mut_raw_socket).unwrap();

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
            self.vec.push(sys::zmq_poller_event_t::default());
            Ok(())
        }
    }

    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::prelude::*;
    ///
    /// let server = Server::new()?;
    /// let mut poller = Poller::new();
    ///
    /// poller.add(&server, 0, NO_POLL)?;
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
    pub fn remove(&mut self, socket: &AsRawSocket) -> Result<(), Error<()>> {
        // This is safe since we don't actually mutate the socket.
        let mut_raw_socket = socket.as_raw_socket() as *mut _;

        if let None = self.map.remove(&mut_raw_socket) {
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
            self.vec.pop().unwrap();

            Ok(())
        }
    }

    pub fn modify<S>(
        &mut self,
        socket: &AsRawSocket,
        events: PollEvents,
    ) -> Result<(), Error<()>> {
        // This is safe since we don't actually mutate the socket.
        let mut_raw_socket = socket.as_raw_socket() as *mut _;

        if let None = self.map.get(&mut_raw_socket) {
            return Err(Error::new(ErrorKind::InvalidInput {
                msg: "cannot modify absent socket",
            }));
        }

        let rc = unsafe {
            sys::zmq_poller_modify(self.poller, mut_raw_socket, events.bits())
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

    pub fn wait(&mut self, timeout: i64) -> Result<PollIter<T>, Error<()>>
    where
        T: Debug,
    {
        let len = self.vec.len();

        let rc = unsafe {
            sys::zmq_poller_wait_all(
                self.poller,
                self.vec.as_mut_ptr(),
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
            let polled: Vec<PollItem<T>> = self
                .vec
                .iter()
                .take(rc as usize)
                .map(|e| e.into())
                .collect();

            Ok(PollIter {
                inner: polled.into_iter(),
            })
        }
    }

    pub fn add_fd(
        &mut self,
        fd: RawFd,
        events: PollEvents,
    ) -> Result<(), Error<()>> {
        unimplemented!()
    }

    pub fn remove_fd(&mut self, fd: RawFd) -> Result<(), Error<()>> {
        unimplemented!()
    }

    pub fn modify_fd(
        &mut self,
        fd: RawFd,
        events: PollEvents,
    ) -> Result<(), Error<()>> {
        unimplemented!()
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
            vec: vec![],
            map: HashMap::default(),
            phantom: PhantomData,
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
    fn test_events() {
        assert_eq!(PollEvents::INCOMING.bits(), sys::ZMQ_POLLIN as c_short);
        assert_eq!(PollEvents::OUTGOING.bits(), sys::ZMQ_POLLOUT as c_short);
    }

    #[test]
    fn test_remove_absent_socket() {
        use crate::prelude::*;

        let server = Server::new().unwrap();

        let mut poller = Poller::<i32>::new();
        let err = poller.remove(&server).unwrap_err();

        match err.kind() {
            ErrorKind::InvalidInput { .. } => (),
            _ => panic!("unexpected error"),
        }
    }
}
