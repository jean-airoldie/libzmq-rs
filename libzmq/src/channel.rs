//! Asynchronous inter-thread communication channels.

use crate::{prelude::*, *};

use std::marker::PhantomData;

/// The sending half of a `Simplex`.
///
/// There can be an arbitrary number of `Sender` associated with
/// a given `Simplex` since it implementes `Clone`. However, they all
/// share the same `FIFO` outgoing queue.
#[derive(Clone, Debug)]
pub struct Sender<T> {
    scatter: Scatter,
    _marker: PhantomData<T>,
}

impl<T: 'static> Sender<T> {
    /// Push a `T` into the outgoing queue.
    ///
    /// This operation might block until `send_timeout` expires, or
    /// block forever if it is not set.
    ///
    /// Note this function allocates at each call.
    ///
    /// # Success
    /// The message was queued and now belongs to ØMQ.
    /// The ownership will be transfered to the receiver,
    /// or, if it is never received, ØMQ will drop it.
    ///
    /// # Error
    /// In case of an error, the message is not queued and
    /// the ownership is returned.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`] (if `send_timeout` expires)
    /// * [`CtxInvalid`]
    /// * [`Interrupted`]
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxInvalid`]: ../enum.ErrorKind.html#variant.CtxInvalid
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    pub fn send(&self, item: T) -> Result<(), Error<T>> {
        let msg = unsafe { Msg::cast_from(item) };
        self.scatter.send(msg).map_err(|mut err| {
            let item = unsafe { err.take_content().unwrap().cast_into() };
            Error::with_content(err.kind(), item)
        })
    }

    /// Try to push a `T` into the outgoing queue without blocking.
    ///
    /// This operation will never block.
    ///
    /// Note that this function allocates at each call.
    ///
    /// # Success
    /// The message was queued and now belongs to ØMQ.
    /// The ownership will be transfered to the receiver,
    /// or, if it is never received, ØMQ will drop it.
    ///
    /// # Error
    /// In case of an error, the message is not queued and
    /// the ownership is returned.
    ///
    /// ## Possible Error Variants
    /// * [`WouldBlock`] (cannot push without blocking)
    /// * [`CtxInvalid`]
    /// * [`Interrupted`]
    ///
    /// [`zmq_msg_send`]: http://api.zeromq.org/master:zmq-msg-send
    /// [`WouldBlock`]: ../enum.ErrorKind.html#variant.WouldBlock
    /// [`CtxInvalid`]: ../enum.ErrorKind.html#variant.CtxInvalid
    /// [`Interrupted`]: ../enum.ErrorKind.html#variant.Interrupted
    pub fn try_send(&self, item: T) -> Result<(), Error<T>> {
        let msg = unsafe { Msg::cast_from(item) };
        self.scatter.try_send(msg).map_err(|mut err| {
            let item = unsafe { err.take_content().unwrap().cast_into() };
            Error::with_content(err.kind(), item)
        })
    }

    pub fn send_hwm(&self) -> Result<i32, Error> {
        self.scatter.send_hwm()
    }

    pub fn set_send_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.scatter.set_send_hwm(hwm)
    }

    pub fn send_timeout(&self) -> Result<Period, Error> {
        self.scatter.send_timeout()
    }

    pub fn set_send_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.scatter.set_send_timeout(period)
    }
}

/// The receiving half of a `Simplex`.
///
/// There can be an arbitrary number of `Receiver` associated with
/// a given `Simplex` since it implementes `Clone`. However, they all
/// share the same `FIFO` incoming queue.
#[derive(Clone, Debug)]
pub struct Receiver<T> {
    gather: Gather,
    _marker: PhantomData<T>,
}

impl<T: 'static> Receiver<T> {
    pub fn recv(&self) -> Result<T, Error> {
        let msg = self.gather.recv_msg()?;
        Ok(unsafe { msg.cast_into() })
    }

    pub fn try_recv(&self) -> Result<T, Error> {
        let msg = self.gather.try_recv_msg()?;
        Ok(unsafe { msg.cast_into() })
    }

    pub fn recv_hwm(&self) -> Result<i32, Error> {
        self.gather.recv_hwm()
    }

    pub fn set_recv_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.gather.set_recv_hwm(hwm)
    }

    pub fn recv_timeout(&self) -> Result<Period, Error> {
        self.gather.recv_timeout()
    }

    pub fn set_recv_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.gather.set_recv_timeout(period)
    }
}

/// Multi-producer, multi-consumer, unidirectional asynchronous channel.
///
/// ```text
/// sender ─┐         ┌─> receiver
/// sender ── simplex ──> receiver
/// sender ─┘         └─> receiver
/// ```
#[derive(Debug)]
pub struct Simplex<T> {
    scatter: Scatter,
    gather: Gather,
    _marker: PhantomData<T>,
}

impl<T: 'static> Simplex<T> {
    /// Create a new `Simplex` using the global context.
    pub fn new() -> Result<Self, Error> {
        Self::with_ctx(Ctx::global())
    }

    /// Creates a new `Simplex` associated with the context of
    /// the handle.
    pub fn with_ctx(handle: CtxHandle) -> Result<Self, Error> {
        let addr = InprocAddr::new_unique();
        let gather = GatherBuilder::new()
            .bind(&addr)
            .with_ctx(handle)
            .map_err(Error::cast)?;
        let scatter = ScatterBuilder::new()
            .connect(&addr)
            .with_ctx(handle)
            .map_err(Error::cast)?;

        Ok(Simplex {
            gather,
            scatter,
            _marker: PhantomData,
        })
    }

    /// Creates another instance of a `Sender`.
    ///
    /// Every `Sender` associated with the `Simplex` share the same outgoing queue.
    pub fn sender(&self) -> Sender<T> {
        Sender {
            scatter: self.scatter.clone(),
            _marker: PhantomData,
        }
    }

    /// Creates another instance of a `Receiver`.
    ///
    /// Every `Receiver` associated with the `Simplex` share the
    /// same incoming queue.
    pub fn receiver(&self) -> Receiver<T> {
        Receiver {
            gather: self.gather.clone(),
            _marker: PhantomData,
        }
    }
}

/// The first half of a `Duplex` channel.
#[derive(Debug, Clone)]
pub struct Front<F, B> {
    client: Client,
    _front: PhantomData<F>,
    _back: PhantomData<B>,
}

impl<F, B> Front<F, B>
where
    F: 'static,
    B: 'static,
{
    pub fn send(&self, msg: F) -> Result<(), Error<F>> {
        let msg = unsafe { Msg::cast_from(msg) };
        self.client.send(msg).map_err(|mut err| {
            let question = unsafe { err.take_content().unwrap().cast_into() };
            Error::with_content(err.kind(), question)
        })
    }

    pub fn try_send(&self, question: F) -> Result<(), Error<F>> {
        let msg = unsafe { Msg::cast_from(question) };
        self.client.try_send(msg).map_err(|mut err| {
            let question = unsafe { err.take_content().unwrap().cast_into() };
            Error::with_content(err.kind(), question)
        })
    }

    pub fn send_hwm(&self) -> Result<i32, Error> {
        self.client.send_hwm()
    }

    pub fn set_send_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.client.set_send_hwm(hwm)
    }

    pub fn send_timeout(&self) -> Result<Period, Error> {
        self.client.send_timeout()
    }

    pub fn set_send_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.client.set_send_timeout(period)
    }

    pub fn recv(&self) -> Result<B, Error> {
        let msg = self.client.recv_msg()?;
        Ok(unsafe { msg.cast_into() })
    }

    pub fn try_recv(&self) -> Result<B, Error> {
        let msg = self.client.try_recv_msg()?;
        Ok(unsafe { msg.cast_into() })
    }

    pub fn recv_hwm(&self) -> Result<i32, Error> {
        self.client.recv_hwm()
    }

    pub fn set_recv_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.client.set_recv_hwm(hwm)
    }

    pub fn recv_timeout(&self) -> Result<Period, Error> {
        self.client.recv_timeout()
    }

    pub fn set_recv_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.client.set_recv_timeout(period)
    }
}

/// The second half of a `Duplex` channel.
#[derive(Debug, Clone)]
pub struct Back<F, B> {
    server: Server,
    id: RoutingId,
    _front: PhantomData<F>,
    _back: PhantomData<B>,
}

impl<F, B> Back<F, B>
where
    F: 'static,
    B: 'static,
{
    pub fn send(&self, answer: B) -> Result<(), Error<B>> {
        let msg = unsafe { Msg::cast_from(answer) };
        self.server.route(msg, self.id).map_err(|mut err| {
            let answer = unsafe { err.take_content().unwrap().cast_into() };
            Error::with_content(err.kind(), answer)
        })
    }

    pub fn try_send(&self, answer: B) -> Result<(), Error<B>> {
        let msg = unsafe { Msg::cast_from(answer) };
        self.server.try_route(msg, self.id).map_err(|mut err| {
            let answer = unsafe { err.take_content().unwrap().cast_into() };
            Error::with_content(err.kind(), answer)
        })
    }

    pub fn send_hwm(&self) -> Result<i32, Error> {
        self.server.send_hwm()
    }

    pub fn set_send_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.server.set_send_hwm(hwm)
    }

    pub fn send_timeout(&self) -> Result<Period, Error> {
        self.server.send_timeout()
    }

    pub fn set_send_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.server.set_send_timeout(period)
    }

    pub fn recv(&self) -> Result<F, Error> {
        let msg = self.server.recv_msg()?;
        let question = unsafe { msg.cast_into() };
        Ok(question)
    }

    pub fn try_recv(&self) -> Result<F, Error> {
        let msg = self.server.try_recv_msg()?;
        let question = unsafe { msg.cast_into() };
        Ok(question)
    }

    pub fn recv_hwm(&self) -> Result<i32, Error> {
        self.server.recv_hwm()
    }

    pub fn set_recv_hwm(&self, hwm: i32) -> Result<(), Error> {
        self.server.set_recv_hwm(hwm)
    }

    pub fn recv_timeout(&self) -> Result<Period, Error> {
        self.server.recv_timeout()
    }

    pub fn set_recv_timeout<P>(&self, period: P) -> Result<(), Error>
    where
        P: Into<Period>,
    {
        self.server.set_recv_timeout(period)
    }
}

/// Multi-producer, multi-consumer, bidirectional asynchronous channel.
/// ```text
/// front <─┐          ┌─> back
/// front <──> duplex <──> back
/// front <─┘          └─> back
/// ```
#[derive(Debug)]
pub struct Duplex<F, B> {
    client: Client,
    server: Server,
    id: RoutingId,
    _front: PhantomData<F>,
    _back: PhantomData<B>,
}

impl<F, B> Duplex<F, B> {
    pub fn new() -> Result<Self, Error> {
        Self::with_ctx(Ctx::global())
    }

    pub fn with_ctx(handle: CtxHandle) -> Result<Self, Error> {
        let addr = InprocAddr::new_unique();
        let server = ServerBuilder::new()
            .bind(&addr)
            .with_ctx(handle)
            .map_err(Error::cast)?;
        let client = ClientBuilder::new()
            .connect(addr)
            .with_ctx(handle)
            .map_err(Error::cast)?;
        client.send("").map_err(Error::cast)?;
        let msg = server.recv_msg()?;
        let id = msg.routing_id().unwrap();

        Ok(Self {
            client,
            server,
            id,
            _front: PhantomData,
            _back: PhantomData,
        })
    }

    pub fn front(&self) -> Front<F, B> {
        Front {
            client: self.client.clone(),
            _front: PhantomData,
            _back: PhantomData,
        }
    }

    pub fn back(&self) -> Back<F, B> {
        Back {
            server: self.server.clone(),
            id: self.id,
            _front: PhantomData,
            _back: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_duplex_round_trip() {
        let duplex = Duplex::new().unwrap();
        let front = duplex.front();
        let back = duplex.back();

        front.send("ok".to_owned()).unwrap();
        back.recv().unwrap();
        back.send(420).unwrap();
        front.recv().unwrap();
    }

    #[test]
    fn test_simplex_trip() {
        let simplex = Simplex::new().unwrap();
        let sender = simplex.sender();
        let receiver = simplex.receiver();

        let payload = "some string".to_owned();
        sender.send(payload).unwrap();
        let result = receiver.recv().unwrap();

        assert_eq!(&result, "some string");
    }
}
