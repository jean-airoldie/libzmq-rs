//! The set of core ØMQ socket traits.

mod raw;
mod recv;
mod send;
pub(crate) mod sockopt;

pub(crate) use raw::*;

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

use crate::{addr::Endpoint, auth::*, error::Error};
use std::{sync::MutexGuard, time::Duration};

/// Methods shared by all thread-safe sockets.
pub trait Socket: GetRawSocket {
    /// Schedules a connection to one or more [`Endpoints`] and then accepts
    /// incoming connections.
    ///
    /// Since ØMQ handles all connections behind the curtain, one cannot know
    /// exactly when the connection is truly established a blocking `send`
    /// or `recv` call is made on that connection.
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// connections that succeeded before the failure.
    ///
    /// See [`zmq_connect`].
    ///
    /// # Usage Contract
    /// * The endpoint(s) must be valid (Endpoint does not do any validation atm).
    /// * The endpoint's protocol must be supported by the socket.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (invalid endpoint)
    /// * [`IncompatTransport`] (transport not supported)
    /// * [`CtxTerminated`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_connect`]: http://api.zeromq.org/master:zmq-connect
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`IncompatTransport`]: ../enum.ErrorKind.html#variant.IncompatTransport
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    fn connect<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        let raw_socket = self.raw_socket();

        for endpoint in endpoints.into_iter().map(E::into) {
            raw_socket
                .connect(&endpoint)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }

        Ok(())
    }

    /// Disconnect the socket from one or more [`Endpoints`].
    ///
    /// Any outstanding messages physically received from the network but not
    /// yet received by the application are discarded. The behaviour for
    /// discarding messages depends on the value of [`linger`].
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// disconnections that succeeded before the failure.
    ///
    /// See [`zmq_disconnect`].
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The endpoint must be already connected to.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (invalid endpoint)
    /// * [`NotFound`] (endpoint not connected to)
    /// * [`CtxTerminated`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_disconnect`]: http://api.zeromq.org/master:zmq-disconnect
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    /// [`linger`]: #method.linger
    fn disconnect<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        let raw_socket = self.raw_socket();

        for endpoint in endpoints.into_iter().map(E::into) {
            raw_socket
                .disconnect(&endpoint)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }

        Ok(())
    }

    /// Schedules a bind to one or more [`Endpoints`] and then accepts
    /// incoming connections.
    ///
    /// As opposed to `connect`, the socket will straight await and start
    /// accepting connections.
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// binds that succeeded before the failure.
    ///
    /// See [`zmq_bind`].
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The transport must be supported by socket type.
    /// * The endpoint must not be in use.
    /// * The endpoint must be local.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (invalid endpoint)
    /// * [`IncompatTransport`] (transport is not supported)
    /// * [`AddrInUse`] (addr already in use)
    /// * [`AddrNotAvailable`] (not local)
    /// * [`CtxTerminated`]
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_bind`]: http://api.zeromq.org/master:zmq-bind
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`IncompatTransport`]: ../enum.ErrorKind.html#variant.IncompatTransport
    /// [`AddrInUse`]: ../enum.ErrorKind.html#variant.AddrInUse
    /// [`AddrNotAvailable`]: ../enum.ErrorKind.html#variant.AddrNotAvailable
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    fn bind<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        let raw_socket = self.raw_socket();

        for endpoint in endpoints.into_iter().map(E::into) {
            raw_socket
                .bind(&endpoint)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }

        Ok(())
    }

    /// Unbinds the socket from one or more [`Endpoints`].
    ///
    /// Any outstanding messages physically received from the network but not
    /// yet received by the application are discarded. The behaviour for
    /// discarding messages depends on the value of [`linger`].
    ///
    /// See [`zmq_unbind`].
    ///
    /// When any of the connection attempt fail, the `Error` will contain the position
    /// of the iterator before the failure. This represents the number of
    /// unbinds that succeeded before the failure.
    ///
    /// When a socket is dropped, it is unbound from all its associated endpoints
    /// so that they become available for binding immediately.
    ///
    /// # Usage Contract
    /// * The endpoint must be valid (Endpoint does not do any validation atm).
    /// * The endpoint must be currently bound.
    ///
    /// # Returned Errors
    /// * [`InvalidInput`] (invalid endpoint)
    /// * [`CtxTerminated`]
    /// * [`NotFound`] (endpoint was not bound to)
    ///
    /// [`Endpoints`]: ../endpoint/enum.Endpoint.html
    /// [`zmq_unbind`]: http://api.zeromq.org/master:zmq-unbind
    /// [`InvalidInput`]: ../enum.ErrorKind.html#variant.InvalidInput
    /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
    /// [`NotFound`]: ../enum.ErrorKind.html#variant.NotFound
    /// [`linger`]: #method.linger
    fn unbind<I, E>(&self, endpoints: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<Endpoint>,
    {
        let mut count = 0;
        let raw_socket = self.raw_socket();

        for endpoint in endpoints.into_iter().map(E::into) {
            raw_socket
                .unbind(&endpoint)
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }

        Ok(())
    }

    /// Retrieve the last endpoint connected or bound to.
    ///
    /// This is the only way to retreive the value of a bound `Dynamic` port.
    ///
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Server, TcpAddr, addr::Endpoint};
    /// use std::convert::TryInto;
    ///
    /// // We create a tcp addr with an unspecified port.
    /// // This port will be assigned by the OS upon connection.
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
    fn last_endpoint(&self) -> Result<Option<Endpoint>, Error> {
        self.raw_socket().last_endpoint()
    }

    /// The interval between sending ZMTP heartbeats.
    fn heartbeat_interval(&self) -> Result<Duration, Error> {
        self.raw_socket().heartbeat_interval()
    }

    /// Sets the interval between sending ZMTP PINGs (aka. heartbeats).
    ///
    /// # Default Value
    /// `None`
    ///
    /// # Applicable Socket Type
    /// All (connection oriented transports)
    fn set_heartbeat_interval<D>(&self, duration: D) -> Result<(), Error>
    where
        D: Into<Duration>,
    {
        self.raw_socket().set_heartbeat_interval(duration.into())
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    fn heartbeat_timeout(&self) -> Result<Duration, Error> {
        self.raw_socket().heartbeat_timeout()
    }

    /// How long to wait before timing-out a connection after sending a
    /// PING ZMTP command and not receiving any traffic.
    ///
    /// # Default Value
    /// `0`. If `heartbeat_interval` is set, then it uses the same value
    /// by default.
    fn set_heartbeat_timeout<D>(&self, duration: D) -> Result<(), Error>
    where
        D: Into<Duration>,
    {
        self.raw_socket().set_heartbeat_timeout(duration.into())
    }

    /// The timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    fn heartbeat_ttl(&self) -> Result<Duration, Error> {
        self.raw_socket().heartbeat_ttl()
    }

    /// Set timeout on the remote peer for ZMTP heartbeats.
    /// If this option and `heartbeat_interval` is not `None` the remote
    /// side shall time out the connection if it does not receive any more
    /// traffic within the TTL period.
    ///
    /// # Default value
    /// `None`
    fn set_heartbeat_ttl<D>(&self, duration: D) -> Result<(), Error>
    where
        D: Into<Duration>,
    {
        self.raw_socket().set_heartbeat_ttl(duration.into())
    }

    /// Returns the linger period for the socket shutdown.
    fn linger(&self) -> Result<Option<Duration>, Error> {
        self.raw_socket().linger()
    }

    /// Sets the linger period for the socket shutdown.
    ///
    /// The linger period determines how long pending messages which have
    /// yet to be sent to a peer shall linger in memory after a socket is
    /// disconnected or dropped.
    ///
    /// A value of `None` means an infinite period.
    ///
    /// # Default Value
    /// 30 secs
    fn set_linger(&self, maybe: Option<Duration>) -> Result<(), Error> {
        self.raw_socket().set_linger(maybe.map(Into::into))
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
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::{prelude::*, Client, auth::*};
    ///
    /// let client = Client::new()?;
    /// assert_eq!(client.mechanism(), Mechanism::Null);
    ///
    /// let server_cert = CurveCert::new_unique();
    /// let creds = CurveClientCreds::new(server_cert.public());
    /// client.set_mechanism(&creds)?;
    ///
    /// if let Mechanism::CurveClient(creds) = client.mechanism() {
    ///     assert_eq!(creds.server(), server_cert.public());
    ///     // Indeed it was automatically generated.
    ///     assert!(creds.client().is_some());
    /// } else {
    ///     unreachable!()
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
        let server_key = {
            if let Mechanism::CurveClient(creds) = mechanism {
                creds.server
            } else {
                unreachable!()
            }
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
    pub(crate) heartbeat_interval: Option<Duration>,
    pub(crate) heartbeat_timeout: Option<Duration>,
    pub(crate) heartbeat_ttl: Option<Duration>,
    pub(crate) linger: Option<Duration>,
    pub(crate) mechanism: Option<Mechanism>,
}

impl SocketConfig {
    pub(crate) fn apply<S: Socket>(
        &self,
        socket: &S,
    ) -> Result<(), Error<usize>> {
        if let Some(value) = self.heartbeat_interval {
            socket.set_heartbeat_interval(value).map_err(Error::cast)?;
        }
        if let Some(value) = self.heartbeat_timeout {
            socket.set_heartbeat_timeout(value).map_err(Error::cast)?;
        }
        if let Some(value) = self.heartbeat_ttl {
            socket.set_heartbeat_ttl(value).map_err(Error::cast)?;
        }
        socket.set_linger(self.linger).map_err(Error::cast)?;
        if let Some(ref mechanism) = self.mechanism {
            socket.set_mechanism(mechanism).map_err(Error::cast)?;
        }
        // We connect as the last step because some socket options
        // only affect subsequent connections.
        if let Some(ref endpoints) = self.connect {
            socket.connect(endpoints)?;
        }
        if let Some(ref endpoints) = self.bind {
            socket.bind(endpoints)?;
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

    fn heartbeat_interval(&self) -> Option<Duration> {
        self.socket_config().heartbeat_interval
    }

    fn set_heartbeat_interval(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().heartbeat_interval = maybe;
    }

    fn heartbeat_timeout(&self) -> Option<Duration> {
        self.socket_config().heartbeat_timeout
    }

    fn set_heartbeat_timeout(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().heartbeat_timeout = maybe;
    }

    fn heartbeat_ttl(&self) -> Option<Duration> {
        self.socket_config().heartbeat_ttl
    }

    fn set_heartbeat_ttl(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().heartbeat_ttl = maybe;
    }

    fn linger(&self) -> Option<Duration> {
        self.socket_config().linger
    }

    fn set_linger(&mut self, maybe: Option<Duration>) {
        self.socket_config_mut().linger = maybe;
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

    fn heartbeat_interval(&mut self, duration: Duration) -> &mut Self {
        self.socket_config_mut()
            .set_heartbeat_interval(Some(duration));
        self
    }

    fn heartbeat_timeout(&mut self, duration: Duration) -> &mut Self {
        self.socket_config_mut()
            .set_heartbeat_timeout(Some(duration));
        self
    }

    fn heartbeat_ttl(&mut self, duration: Duration) -> &mut Self {
        self.socket_config_mut().set_heartbeat_ttl(Some(duration));
        self
    }

    fn linger(&mut self, maybe: Option<Duration>) -> &mut Self {
        self.socket_config_mut().set_linger(maybe);
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
