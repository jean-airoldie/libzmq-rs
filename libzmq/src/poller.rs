use crate::{
    error::{msg_from_errno, Error, ErrorKind},
    socket::AsRawSocket,
};

use libzmq_sys as sys;
use sys::errno;

use hashbrown::HashMap;

use bitflags::bitflags;

use std::{
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
        const POLL_IN = 0b00_000_001;
        const POLL_OUT = 0b00_000_010;
    }
}

pub const NO_POLL: PollEvents = PollEvents::NO_POLL;
pub const POLL_IN: PollEvents = PollEvents::POLL_IN;
pub const POLL_OUT: PollEvents = PollEvents::POLL_OUT;

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

pub struct PollItem<'a, T> {
    pub events: PollEvents,
    pub user_data: &'a T,
}

impl<'a, T> From<&'a sys::zmq_poller_event_t> for PollItem<'a, T> {
    fn from(poll: &'a sys::zmq_poller_event_t) -> Self {
        let events = PollEvents::from_bits(poll.events).unwrap();
        // This is a pointer into Poller.user_data, thus it is linked
        // to the 'a lifetime. Thus this is safe.
        let user_data =
            unsafe { std::mem::transmute::<*mut c_void, &T>(poll.user_data) };

        Self { events, user_data }
    }
}

///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::prelude::*;
///
/// #[derive(PartialEq)]
/// enum Item {
///   First,
///   Second,
/// };
///
/// let endpoint: Endpoint = "inproc://test".parse().unwrap();
///
/// let radio = Radio::new()?;
/// radio.bind(&endpoint)?;
/// radio.set_no_drop(true)?;
///
/// let first = Dish::new()?;
/// first.connect(&endpoint)?;
/// first.join("some group")?;
///
/// let second = Dish::new()?;
/// second.connect(endpoint)?;
/// second.join("some group")?;
///
/// let mut poller = Poller::new();
/// poller.add(&radio, Item::First, POLL_OUT)?;
/// poller.add(&first, Item::Second, POLL_IN)?;
/// poller.add(&second, Item::Second, POLL_IN)?;
///
/// let mut msg: Msg = "message".into();
/// msg.set_group("some group");
/// radio.send(msg)?;
///
/// for poll in poller.wait(0)? {
///   match poll.user_data {
///     Item::First => {
///       // The radio socket is ready to send.
///       assert_eq!(poll.events, POLL_OUT);
///     }
///     Item::Second => {
///       // One of the dish sockets is ready to receive.
///       assert_eq!(poll.events, POLL_IN);
///     }
///   }
/// }
/// #
/// #     Ok(())
/// # }
/// ```
pub struct Poller<T> {
    poller: *mut c_void,
    vec: Vec<sys::zmq_poller_event_t>,
    user_data: Vec<T>,
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
    ) -> Result<(), Error<()>> {
        self.user_data.push(user_data);
        let user_data = self.user_data.iter_mut().last().unwrap();
        let mut_user_data_ptr = user_data as *mut T as *mut _;

        let mut_raw_socket = socket.as_raw_socket() as *mut _;

        let rc = unsafe {
            sys::zmq_poller_add(
                self.poller,
                mut_raw_socket,
                mut_user_data_ptr,
                events.bits(),
            )
        };

        if rc == -1 {
            self.user_data.pop().unwrap();
            assert_eq!(self.user_data.len(), self.vec.len());

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
        let mut_raw_socket = socket.as_raw_socket() as *mut _;

        let rc = unsafe { sys::zmq_poller_remove(self.poller, mut_raw_socket) };

        if rc == -1 {
            let errno = unsafe { sys::zmq_errno() };

            let err = {
                match errno {
                    errno::EINVAL => Error::new(ErrorKind::InvalidInput {
                        msg: "cannot remove absent socket",
                    }),
                    errno::ENOTSOCK => panic!("invalid socket"),
                    _ => panic!(msg_from_errno(errno)),
                }
            };

            Err(err)
        } else {
            self.vec.pop().unwrap();
            self.user_data.pop().unwrap();

            Ok(())
        }
    }

    pub fn modify<S>(
        &mut self,
        socket: &S,
        events: PollEvents,
    ) -> Result<(), Error<()>> {
        unimplemented!()
    }

    pub fn wait(&mut self, timeout: i64) -> Result<PollIter<T>, Error<()>> {
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
            println!("{:?}", self.vec);
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
            user_data: vec![],
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
        assert_eq!(PollEvents::POLL_IN.bits(), sys::ZMQ_POLLIN as c_short);
        assert_eq!(PollEvents::POLL_OUT.bits(), sys::ZMQ_POLLOUT as c_short);
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

    #[test]
    fn test_poller() {
        use crate::prelude::*;

        #[derive(PartialEq)]
        enum Item {
            First,
            Second,
        };

        let endpoint: Endpoint = "inproc://test".parse().unwrap();

        let server = Server::new().unwrap();
        server.bind(&endpoint).unwrap();

        let client = Client::new().unwrap();
        client.connect(&endpoint).unwrap();

        let mut poller = Poller::new();
        poller.add(&server, Item::First, POLL_IN).unwrap();
        poller.add(&client, Item::Second, POLL_IN).unwrap();

        client.send("msg").unwrap();

        for _ in 0..100 {
            for poll in poller.wait(0).unwrap() {
                match poll.user_data {
                    Item::First => {
                        let msg = server.recv_msg().unwrap();
                        server.send(msg).unwrap();
                    }
                    Item::Second => {
                        let msg = client.recv_msg().unwrap();
                        client.send(msg).unwrap();
                    }
                }
            }
        }
    }
}
