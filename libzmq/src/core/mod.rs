//! The set of core ØMQ socket traits.

mod heartbeat;
mod raw;
mod recv;
mod send;
pub(crate) mod sockopt;

pub(crate) use raw::*;

pub use heartbeat::*;
pub use recv::*;
pub use send::*;

/// Prevent users from implementing the AsRawSocket trait.
mod private {
    use super::*;
    use crate::socket::*;

    pub trait Sealed {}
    impl Sealed for SocketConfig {}
    impl Sealed for SendConfig {}
    impl Sealed for RecvConfig {}
    impl Sealed for HeartbeatingConfig {}
    impl Sealed for Client {}
    impl Sealed for ClientConfig {}
    impl Sealed for ClientBuilder {}
    impl Sealed for Server {}
    impl Sealed for ServerConfig {}
    impl Sealed for ServerBuilder {}
    impl Sealed for Radio {}
    impl Sealed for RadioConfig {}
    impl Sealed for RadioBuilder {}
    impl Sealed for Dish {}
    impl Sealed for DishConfig {}
    impl Sealed for DishBuilder {}
    impl Sealed for Scatter {}
    impl Sealed for ScatterConfig {}
    impl Sealed for ScatterBuilder {}
    impl Sealed for Gather {}
    impl Sealed for GatherConfig {}
    impl Sealed for GatherBuilder {}
    impl Sealed for SocketType {}

    // Pub crate
    use crate::old::OldSocket;
    impl Sealed for OldSocket {}
}

use crate::{addr::Endpoint, auth::*, Error};

use humantime_serde::Serde;
use serde::{Deserialize, Serialize};

use std::{sync::MutexGuard, time::Duration};

const DEFAULT_HWM: i32 = 1000;
const DEFAULT_BATCH_SIZE: i32 = 8192;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "Option<i32>")]
#[serde(into = "Option<i32>")]
pub(crate) struct HighWaterMark(i32);

impl Default for HighWaterMark {
    fn default() -> Self {
        HighWaterMark(DEFAULT_HWM)
    }
}

impl From<i32> for HighWaterMark {
    fn from(i: i32) -> Self {
        Self(i)
    }
}

impl From<HighWaterMark> for i32 {
    fn from(hwm: HighWaterMark) -> i32 {
        hwm.0
    }
}

impl From<Option<i32>> for HighWaterMark {
    fn from(v: Option<i32>) -> Self {
        match v {
            Some(v) => Self(v),
            None => Self::default(),
        }
    }
}

impl From<HighWaterMark> for Option<i32> {
    fn from(hwm: HighWaterMark) -> Self {
        Some(hwm.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(from = "Option<i32>")]
#[serde(into = "Option<i32>")]
pub(crate) struct BatchSize(i32);

impl Default for BatchSize {
    fn default() -> Self {
        BatchSize(DEFAULT_BATCH_SIZE)
    }
}

impl From<i32> for BatchSize {
    fn from(v: i32) -> Self {
        BatchSize(v)
    }
}

impl From<BatchSize> for i32 {
    fn from(size: BatchSize) -> i32 {
        size.0
    }
}

impl From<Option<i32>> for BatchSize {
    fn from(v: Option<i32>) -> Self {
        match v {
            Some(v) => Self(v),
            None => Self::default(),
        }
    }
}

impl From<BatchSize> for Option<i32> {
    fn from(size: BatchSize) -> Self {
        Some(size.0)
    }
}

/// Represents a period of time.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "Serde<Option<Duration>>")]
#[serde(into = "Serde<Option<Duration>>")]
pub enum Period {
    /// A unbounded period of time.
    Infinite,
    /// A bounded period of time.
    Finite(Duration),
}

pub use Period::*;

impl Default for Period {
    fn default() -> Self {
        Infinite
    }
}

#[doc(hidden)]
impl From<Period> for Option<Duration> {
    fn from(period: Period) -> Self {
        match period {
            Finite(duration) => Some(duration),
            Infinite => None,
        }
    }
}

#[doc(hidden)]
impl From<Option<Duration>> for Period {
    fn from(option: Option<Duration>) -> Self {
        match option {
            None => Infinite,
            Some(duration) => Finite(duration),
        }
    }
}

#[doc(hidden)]
impl From<Serde<Option<Duration>>> for Period {
    fn from(serde: Serde<Option<Duration>>) -> Self {
        match serde.into_inner() {
            None => Infinite,
            Some(duration) => Finite(duration),
        }
    }
}

#[doc(hidden)]
impl From<Period> for Serde<Option<Duration>> {
    fn from(period: Period) -> Self {
        let inner = match period {
            Finite(duration) => Some(duration),
            Infinite => None,
        };

        Serde::from(inner)
    }
}

/// Methods shared by all thread-safe sockets.
pub trait Socket: GetRawSocket {
    /// Schedules a connection to a [`Endpoint`].
    ///
    /// Since ØMQ handles all connections behind the curtain, one cannot know
    /// exactly when the connection is truly established a blocking `send`
    /// or `recv` call is made on that connection.
    ///
    /// # Usage Contract
    /// * The endpoint's protocol must be supported by the socket.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (transport incompatible or not supported)
    /// * [`InvalidCtx`]
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Client, TcpAddr};
    /// use std::convert::TryInto;
    ///
    /// let addr: TcpAddr = "127.0.0.1:420".try_into()?;
    ///
    /// let client = Client::new()?;
    /// client.connect(&addr)?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    fn connect<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: Into<Endpoint>,
    {
        self.raw_socket().connect(&endpoint.into())
    }

    /// Schedules a bind to a [`Endpoint`] and then accepts
    /// incoming connections.
    ///
    /// As opposed to `connect`, the socket will straight await and start
    /// accepting connections.
    ///
    /// # Usage Contract
    /// * The transport must be supported by the socket type.
    /// * The endpoint must not be in use.
    /// * The endpoint must be local.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (transport incompatible or not supported)
    /// * [`AddrInUse`] (addr already in use)
    /// * [`AddrNotAvailable`] (addr not local)
    /// * [`InvalidCtx`]
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Server, TcpAddr};
    /// use std::convert::TryInto;
    ///
    /// // Use a system-assigned port.
    /// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
    ///
    /// let server = Server::new()?;
    /// server.bind(addr)?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`AddrInUse`]: ../enum.ErrorKind.html#variant.AddrInUse
    /// [`AddrNotAvailable`]: ../enum.ErrorKind.html#variant.AddrNotAvailable
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    fn bind<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: Into<Endpoint>,
    {
        self.raw_socket().bind(&endpoint.into())
    }

    /// Disconnect the socket from a [`Endpoint`].
    ///
    /// ## Disconnect from a connected endpoint
    /// The socket stops receiving and sending messages to the remote.
    /// The incoming and outgoing queue of the socket associated to the endpoint
    /// are discarded. However, the remote server might still have outstanding
    /// messages from the socket sent prior to the disconnection in its
    /// incoming queue.
    ///
    /// # Usage Contract
    /// * The endpoint must be currently connected to.
    ///
    /// # Returned Errors
    /// * [`NotFound`] (endpoint was not connected to)
    /// * [`InvalidCtx`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    fn disconnect<E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: Into<Endpoint>,
    {
        self.raw_socket().disconnect(&endpoint.into())
    }

    /// Unbind the socket from a [`Endpoint`].
    ///
    /// ## Disconnect from a bound endpoint
    /// The socket stops receiving and sending messages to peers connected to
    /// the now unbound endpoint. The outgoing queue of the socket associated
    /// to the endpoint is discarded, but the incoming queue is kept.
    ///
    /// # Usage Contract
    /// * The endpoint must be currently bound to.
    ///
    /// # Returned Errors
    /// * [`NotFound`] (endpoint was not bound to)
    /// * [`InvalidCtx`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`InvalidCtx`]: ../enum.ErrorKind.html#variant.InvalidCtx
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    fn unbind<I, E>(&self, endpoint: E) -> Result<(), Error>
    where
        E: Into<Endpoint>,
    {
        self.raw_socket().unbind(&endpoint.into())
    }

    /// Retrieve the last endpoint connected or bound to.
    ///
    /// This is the only way to retrieve the assigned value of an
    /// [`Unspecified`] port.
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Server, TcpAddr, addr::Endpoint};
    ///
    /// // We create a tcp addr with an unspecified port.
    /// // This port will be assigned by the OS when binding.
    /// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
    /// assert!(addr.host().port().is_unspecified());
    ///
    /// let server = Server::new()?;
    /// assert!(server.last_endpoint()?.is_none());
    ///
    /// server.bind(&addr)?;
    ///
    /// if let Endpoint::Tcp(tcp) = server.last_endpoint()?.unwrap() {
    ///     // The port was indeed assigned by the OS.
    ///     assert!(tcp.host().port().is_specified());
    /// } else {
    ///     unreachable!();
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`Unspecified`]: ../addr/enum.Port.html#variant.Unspecified
    fn last_endpoint(&self) -> Result<Option<Endpoint>, Error> {
        self.raw_socket().last_endpoint()
    }

    /// Returns the socket's [`Mechanism`].
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Server, auth::Mechanism};
    ///
    /// let server = Server::new()?;
    /// assert_eq!(server.mechanism(), Mechanism::Null);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`Mechanism`]: ../auth/enum.Mechanism.html
    fn mechanism(&self) -> Mechanism {
        self.raw_socket().mechanism().lock().unwrap().to_owned()
    }

    /// Set the socket's [`Mechanism`].
    ///
    /// # Feature Flags
    ///
    /// Note that `Mechanism::CurveClient` and `Mechanism::CurveServer` require
    /// the feature flag "curve" to be enabled, and will panic if used otherwise.
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// #[cfg(feature = "curve")] {
    ///     use libzmq::{prelude::*, Client, auth::*};
    ///
    ///     let client = Client::new()?;
    ///     assert_eq!(client.mechanism(), Mechanism::Null);
    ///
    ///     let server_cert = CurveCert::new_unique();
    ///     // We do not specify a client certificate, so it
    ///     // will be automatically generated.
    ///     let creds = CurveClientCreds::new(server_cert.public());
    ///
    ///     client.set_mechanism(&creds)?;
    ///
    ///     if let Mechanism::CurveClient(creds) = client.mechanism() {
    ///         assert_eq!(creds.server(), server_cert.public());
    ///         assert!(creds.cert().is_some());
    ///     } else {
    ///         unreachable!()
    ///     }
    /// }
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// [`Mechanism`]: ../auth/enum.Mechanism.html
    fn set_mechanism<M>(&self, mechanism: M) -> Result<(), Error>
    where
        M: Into<Mechanism>,
    {
        let raw_socket = self.raw_socket();
        let mechanism = mechanism.into();
        let mutex = raw_socket.mechanism().lock().unwrap();

        set_mechanism(raw_socket, mechanism, mutex)
    }
}

fn set_mechanism(
    raw_socket: &RawSocket,
    mut mechanism: Mechanism,
    mut mutex: MutexGuard<Mechanism>,
) -> Result<(), Error> {
    if *mutex == mechanism {
        return Ok(());
    }

    // Undo the previous mechanism.
    match &*mutex {
        Mechanism::Null => (),
        Mechanism::PlainClient(_) => {
            raw_socket.set_username(None)?;
            raw_socket.set_password(None)?;
        }
        Mechanism::PlainServer => {
            raw_socket.set_plain_server(false)?;
        }
        Mechanism::CurveClient(_) => {
            raw_socket.set_curve_server_key(None)?;
            raw_socket.set_curve_public_key(None)?;
            raw_socket.set_curve_secret_key(None)?;
        }
        Mechanism::CurveServer(_) => {
            raw_socket.set_curve_secret_key(None)?;
            raw_socket.set_curve_server(false)?;
        }
    }

    // Check if we need to generate a client cert.
    let mut missing_client_cert = false;
    if let Mechanism::CurveClient(creds) = &mechanism {
        if creds.client.is_none() {
            missing_client_cert = true;
        }
    }

    // Generate a client certificate if it was not supplied.
    if missing_client_cert {
        let cert = CurveCert::new_unique();
        let server_key = if let Mechanism::CurveClient(creds) = mechanism {
            creds.server
        } else {
            unreachable!()
        };

        let creds = CurveClientCreds {
            client: Some(cert),
            server: server_key,
        };
        mechanism = Mechanism::CurveClient(creds);
    }

    // Apply the new mechanism.
    match &mechanism {
        Mechanism::Null => (),
        Mechanism::PlainClient(creds) => {
            raw_socket.set_username(Some(&creds.username))?;
            raw_socket.set_password(Some(&creds.password))?;
        }
        Mechanism::PlainServer => {
            raw_socket.set_plain_server(true)?;
        }
        Mechanism::CurveClient(creds) => {
            let server_key: BinCurveKey = (&creds.server).into();
            raw_socket.set_curve_server_key(Some(&server_key))?;

            // Cannot fail since we would have generated a cert.
            let cert = creds.client.as_ref().unwrap();
            let public_key: BinCurveKey = cert.public().into();
            raw_socket.set_curve_public_key(Some(&public_key))?;
            let secret_key: BinCurveKey = cert.secret().into();
            raw_socket.set_curve_secret_key(Some(&secret_key))?;
        }
        Mechanism::CurveServer(creds) => {
            let secret_key: BinCurveKey = (&creds.secret).into();
            raw_socket.set_curve_secret_key(Some(&secret_key))?;
            raw_socket.set_curve_server(true)?;
        }
    }

    // Update mechanism
    *mutex = mechanism;
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[doc(hidden)]
pub struct SocketConfig {
    pub(crate) connect: Option<Vec<Endpoint>>,
    pub(crate) bind: Option<Vec<Endpoint>>,
    pub(crate) mechanism: Option<Mechanism>,
}

impl SocketConfig {
    pub(crate) fn apply<S: Socket>(&self, socket: &S) -> Result<(), Error> {
        if let Some(ref mechanism) = self.mechanism {
            socket.set_mechanism(mechanism)?;
        }
        // We connect as the last step because some socket options
        // only affect subsequent connections.
        if let Some(ref endpoints) = self.connect {
            for endpoint in endpoints {
                socket.connect(endpoint)?;
            }
        }
        if let Some(ref endpoints) = self.bind {
            for endpoint in endpoints {
                socket.bind(endpoint)?;
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub trait GetSocketConfig: private::Sealed {
    fn socket_config(&self) -> &SocketConfig;

    fn socket_config_mut(&mut self) -> &mut SocketConfig;
}

impl GetSocketConfig for SocketConfig {
    fn socket_config(&self) -> &SocketConfig {
        self
    }

    fn socket_config_mut(&mut self) -> &mut SocketConfig {
        self
    }
}

/// A set of provided methods for a socket configuration.
pub trait ConfigureSocket: GetSocketConfig {
    fn connect(&self) -> Option<&[Endpoint]> {
        self.socket_config().connect.as_ref().map(Vec::as_slice)
    }

    fn set_connect<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let maybe: Option<Vec<Endpoint>> =
            maybe.map(|e| e.into_iter().map(E::into).collect());
        self.socket_config_mut().connect = maybe;
    }

    fn bind(&self) -> Option<&[Endpoint]> {
        self.socket_config().bind.as_ref().map(Vec::as_slice)
    }

    fn set_bind<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let maybe: Option<Vec<Endpoint>> =
            maybe.map(|e| e.into_iter().map(E::into).collect());
        self.socket_config_mut().bind = maybe;
    }

    fn mechanism(&self) -> Option<&Mechanism> {
        self.socket_config().mechanism.as_ref()
    }

    fn set_mechanism(&mut self, maybe: Option<Mechanism>) {
        self.socket_config_mut().mechanism = maybe;
    }
}

impl ConfigureSocket for SocketConfig {}

/// A set of provided methods for a socket builder.
pub trait BuildSocket: GetSocketConfig + Sized {
    fn connect<I, E>(&mut self, endpoints: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        self.socket_config_mut().set_connect(Some(endpoints));
        self
    }

    fn bind<I, E>(&mut self, endpoints: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        self.socket_config_mut().set_bind(Some(endpoints));
        self
    }

    fn mechanism<M>(&mut self, mechanism: M) -> &mut Self
    where
        M: Into<Mechanism>,
    {
        self.socket_config_mut()
            .set_mechanism(Some(mechanism.into()));
        self
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_disconnect_connection() {
        use crate::{prelude::*, *};
        use std::{convert::TryInto, thread, time::Duration};

        // Use a system-assigned port.
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server =
            ServerBuilder::new().bind(addr).recv_hwm(1).build().unwrap();

        let bound = server.last_endpoint().unwrap();

        let client = ClientBuilder::new().connect(&bound).build().unwrap();

        for _ in 0..3 {
            client.send("").unwrap();
        }

        // Confirm that we can indeed recv messages.
        let mut msg = server.recv_msg().unwrap();

        let id = msg.routing_id().unwrap();
        server.route("", id).unwrap();

        // Since the server has a recv high water mark of 1,
        // this means that is only one outstanding message.
        client.disconnect(bound).unwrap();
        // Let the client some time to disconnect.
        thread::sleep(Duration::from_millis(50));

        // The client's incoming message queue was discarded.
        client.try_recv(&mut msg).unwrap_err();

        // We received this message before the disconnection.
        server.recv(&mut msg).unwrap();
        // The client's outgoing message queue was discarded.
        server.try_recv(&mut msg).unwrap_err();
    }

    #[test]
    fn test_disconnect_bind() {
        use crate::{prelude::*, *};
        use std::{convert::TryInto, thread, time::Duration};

        // Use a system-assigned port.
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new().bind(addr).build().unwrap();

        let bound = server.last_endpoint().unwrap();

        let client = ClientBuilder::new().connect(&bound).build().unwrap();

        for _ in 0..3 {
            client.send("").unwrap();
        }

        // Confirm that we can indeed recv messages.
        let mut msg = server.recv_msg().unwrap();

        let id = msg.routing_id().unwrap();
        server.route("", id).unwrap();

        server.disconnect(bound).unwrap();
        // Let the server some time to disconnect.
        thread::sleep(Duration::from_millis(50));

        // The client can recv messages sent before the disconnection.
        client.recv(&mut msg).unwrap();

        // The server's incoming queue was not discarded.
        for _ in 0..2 {
            server.recv(&mut msg).unwrap();
        }

        client.send("").unwrap();
        // However the socket no longer accepts new messages.
        server.try_recv(&mut msg).unwrap_err();

        // And we can't reply to the client anymore.
        let err = server.route("", id).unwrap_err();
        match err.kind() {
            ErrorKind::HostUnreachable => (),
            _ => panic!(),
        }
    }
}
