use super::{server::COMMAND_ENDPOINT, *};
use crate::{addr::IntoIpAddrs, prelude::*, socket::*, *};

use serde::{Deserialize, Serialize};

use std::net::{IpAddr, Ipv6Addr};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum AuthRequest {
    AddBlacklist(Ipv6Addr),
    RemoveBlacklist(Ipv6Addr),
    SetBlacklist(Vec<Ipv6Addr>),
    AddWhitelist(Ipv6Addr),
    RemoveWhitelist(Ipv6Addr),
    SetWhitelist(Vec<Ipv6Addr>),
    AddPlainRegistry(PlainClientCreds),
    RemovePlainRegistry(String),
    SetPlainRegistry(Vec<PlainClientCreds>),
    AddCurveRegistry(CurvePublicKey),
    RemoveCurveRegistry(CurvePublicKey),
    SetCurveRegistry(Vec<CurvePublicKey>),
    SetCurveAuth(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum AuthReply {
    Success,
}

fn into_ipv6(ip: IpAddr) -> Ipv6Addr {
    match ip {
        IpAddr::V4(ipv4) => ipv4.to_ipv6_mapped(),
        IpAddr::V6(ipv6) => ipv6,
    }
}

/// A client to configure the `AuthServer`.
///
/// There can be multiple `AuthClient` associated with the same `AuthServer`.
///
/// # Example
///
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::{prelude::*, auth::*, *};
/// use std::{time::Duration};
///
/// let server_cert = CurveCert::new_unique();
/// let client_cert = CurveCert::new_unique();
///
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let server_creds = CurveServerCreds::new(server_cert.secret());
///
/// // Creates a server using the `CurveServer` mechanism. Since `CURVE`
/// // authentication is enabled by default, only sockets whose public key
/// // is in the whitelist will be allowed to connect.
/// let server = ServerBuilder::new()
///     .bind(&addr)
///     .mechanism(server_creds)
///     .recv_timeout(Duration::from_millis(200))
///     .build()?;
///
/// // We need to tell the `AuthServer` to allow the client's public key.
/// let _ = AuthBuilder::new().curve_registry(client_cert.public()).build()?;
///
/// let bound = server.last_endpoint()?;
///
/// let client_creds = CurveClientCreds::new(server_cert.public())
///     .add_cert(client_cert);
///
/// // Creates a server using the `CurveServer` mechanism. Since `CURVE`
/// let client = ClientBuilder::new()
///     .mechanism(client_creds)
///     .connect(bound)
///     .build()?;
///
/// // The handshake is successfull so we can now send and receive messages.
/// client.send("").unwrap();
/// server.recv_msg().unwrap();
/// #
/// #     Ok(())
/// # }
/// ```
pub struct AuthClient {
    client: Client,
}

impl AuthClient {
    /// Create a `AuthClient` connected to `AuthServer` associated
    /// with the default global `Ctx`.
    pub fn new() -> Result<Self, Error> {
        Self::with_ctx(Ctx::global())
    }

    /// Create a `AuthClient` connected to `AuthServer` associated
    /// with the give `Ctx`.
    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let client = ClientBuilder::new()
            .connect(&*COMMAND_ENDPOINT)
            .with_ctx(ctx)
            .map_err(Error::cast)?;

        Ok(AuthClient { client })
    }

    fn request(&self, request: &AuthRequest) -> Result<(), Error> {
        let ser = bincode::serialize(request).unwrap();

        self.client.send(ser).map_err(Error::cast)?;
        let msg = self.client.recv_msg()?;
        let reply: AuthReply = bincode::deserialize(msg.as_bytes()).unwrap();

        assert_eq!(reply, AuthReply::Success);
        Ok(())
    }

    /// Add the ips to the `AuthServer`'s blacklist.
    ///
    /// Blacklisted ips will be denied access.
    pub fn add_blacklist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ipv6 in ips.into_ip_addrs().map(into_ipv6) {
            self.request(&AuthRequest::AddBlacklist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the ips from the `AuthServer`'s blacklist, if
    /// they are present.
    pub fn remove_blacklist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ipv6 in ips.into_ip_addrs().map(into_ipv6) {
            self.request(&AuthRequest::RemoveBlacklist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Set the ips in the `AuthServer`'s blacklist.
    ///
    /// Blacklisted ips will be denied access.
    pub fn set_blacklist<I>(&self, ips: I) -> Result<(), Error>
    where
        I: IntoIpAddrs,
    {
        let ips: Vec<Ipv6Addr> = ips.into_ip_addrs().map(into_ipv6).collect();

        self.request(&AuthRequest::SetBlacklist(ips))
            .map_err(Error::cast)
    }

    /// Add the ips to the `AuthServer`'s whitelist.
    ///
    /// If the whitelist is not empty, only ips in present
    /// in the whitelist are allowed. The whitelist takes precedence
    /// over the blacklist.
    pub fn add_whitelist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ipv6 in ips.into_ip_addrs().map(into_ipv6) {
            self.request(&AuthRequest::AddWhitelist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the ips from the `AuthServer`'s whitelist, if it
    /// is present.
    pub fn remove_whitelist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ipv6 in ips.into_ip_addrs().map(into_ipv6) {
            self.request(&AuthRequest::RemoveWhitelist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Set the ips in the `AuthServer`'s whitelist.
    ///
    /// If the whitelist is not empty, only ips in present
    /// in the whitelist are allowed. The whitelist takes precedence
    /// over the blacklist.
    pub fn set_whitelist<I>(&self, ips: I) -> Result<(), Error>
    where
        I: IntoIpAddrs,
    {
        let ips: Vec<Ipv6Addr> = ips.into_ip_addrs().map(into_ipv6).collect();

        self.request(&AuthRequest::SetWhitelist(ips))
    }

    /// Add the credentials to the `AuthServer`'s plain registry.
    ///
    /// Only credentials present in the registry can successfully authenticate.
    pub fn add_plain_registry<I, E>(&self, iter: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        let mut count = 0;

        for creds in iter.into_iter().map(E::into) {
            self.request(&AuthRequest::AddPlainRegistry(creds))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the credentials with the given usernames from the plain registry.
    pub fn remove_plain_registry<I, E>(
        &self,
        usernames: I,
    ) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<String>,
    {
        let mut count = 0;

        for username in usernames.into_iter().map(E::into) {
            self.request(&AuthRequest::RemovePlainRegistry(username))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Set the credentials in the `AuthServer`'s plain registry.
    ///
    /// Only credentials present in the registry can successfully authenticate.
    pub fn set_plain_registry<I, E>(&self, creds: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        let plain_creds: Vec<PlainClientCreds> =
            creds.into_iter().map(E::into).collect();

        self.request(&AuthRequest::SetPlainRegistry(plain_creds))
    }

    /// Add the curve keys to the curve registry.
    ///
    /// Only public keys present in the whitelist are allowed to authenticate
    /// via the `CURVE` mechanism.
    pub fn add_curve_registry<I, E>(&self, keys: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<CurvePublicKey>,
    {
        let mut count = 0;

        for key in keys.into_iter().map(E::into) {
            self.request(&AuthRequest::AddCurveRegistry(key))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the given public keys from the `AuthServer`'s curve registry
    /// if they are present.
    pub fn remove_curve_registry<I, E>(
        &self,
        keys: I,
    ) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<CurvePublicKey>,
    {
        let mut count = 0;

        for key in keys.into_iter().map(E::into) {
            self.request(&AuthRequest::RemoveCurveRegistry(key))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Set the public keys in the `AuthServer`'s curve registry.
    pub fn set_curve_registry<I, E>(&self, keys: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = E>,
        E: Into<CurvePublicKey>,
    {
        let keys: Vec<CurvePublicKey> = keys.into_iter().map(E::into).collect();

        self.request(&AuthRequest::SetCurveRegistry(keys))
    }

    /// Sets whether to use authentication for the `CURVE` mechanism.
    ///
    /// If it is set to `true`, then only sockets whose public key is present
    /// in the whitelist will be allowed to authenticate. Otherwise all sockets
    /// authenticate successfully.
    pub fn set_curve_auth(&self, enabled: bool) -> Result<(), Error> {
        self.request(&AuthRequest::SetCurveAuth(enabled))
    }
}

/// A Configuration of the `AuthServer`.
///
/// A `AuthClient` must be used to communicate this configuration with the
/// server.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthConfig {
    blacklist: Option<Vec<IpAddr>>,
    whitelist: Option<Vec<IpAddr>>,
    plain_registry: Option<Vec<PlainClientCreds>>,
    curve_registry: Option<Vec<CurvePublicKey>>,
    curve_auth: Option<bool>,
}

impl AuthConfig {
    /// Create an empty `AuthConfig`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to build a `AuthClient` and send the configuration
    /// to the `AuthServer` associated with the default global `Ctx`.
    pub fn build(&self) -> Result<AuthClient, Error> {
        self.with_ctx(Ctx::global())
    }

    /// Attempts to build a `AuthClient` and send the configuration
    /// to the `AuthServer` associated with the given `Ctx`.
    pub fn with_ctx<C>(&self, ctx: C) -> Result<AuthClient, Error>
    where
        C: Into<Ctx>,
    {
        let client = AuthClient::with_ctx(ctx)?;
        self.apply(&client)?;

        Ok(client)
    }

    /// Apply the configuration to the `AuthClient` which will send
    /// it to its associated `AuthServer`.
    pub fn apply(&self, client: &AuthClient) -> Result<(), Error> {
        if let Some(ref blacklist) = self.blacklist {
            client.set_blacklist(blacklist)?;
        }
        if let Some(ref whitelist) = self.whitelist {
            client.set_whitelist(whitelist)?;
        }
        if let Some(ref creds) = self.plain_registry {
            client.set_plain_registry(creds)?;
        }
        if let Some(ref keys) = self.curve_registry {
            client.set_curve_registry(keys)?;
        }
        if let Some(enabled) = self.curve_auth {
            client.set_curve_auth(enabled).map_err(Error::cast)?;
        }

        Ok(())
    }

    pub fn set_blacklist<I>(&mut self, maybe: Option<I>)
    where
        I: IntoIpAddrs,
    {
        let maybe: Option<Vec<IpAddr>> =
            maybe.map(|i| i.into_ip_addrs().collect());
        self.blacklist = maybe;
    }

    pub fn set_whitelist<I>(&mut self, maybe: Option<I>)
    where
        I: IntoIpAddrs,
    {
        let maybe: Option<Vec<IpAddr>> =
            maybe.map(|i| i.into_ip_addrs().collect());
        self.whitelist = maybe;
    }

    pub fn set_plain_registry<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        let maybe: Option<Vec<PlainClientCreds>> =
            maybe.map(|e| e.into_iter().map(E::into).collect());
        self.plain_registry = maybe;
    }

    pub fn set_curve_registry<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<CurvePublicKey>,
    {
        let maybe: Option<Vec<CurvePublicKey>> =
            maybe.map(|e| e.into_iter().map(E::into).collect());
        self.curve_registry = maybe;
    }

    pub fn set_curve_auth(&mut self, maybe: Option<bool>) {
        self.curve_auth = maybe;
    }
}

/// A builder for a `AuthClient`.
///
/// Creates a `AuthClient` and sends the configuration to the associated
/// `AuthServer`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthBuilder {
    inner: AuthConfig,
}

impl AuthBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<AuthClient, Error> {
        self.inner.build()
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<AuthClient, Error>
    where
        C: Into<Ctx>,
    {
        self.inner.with_ctx(ctx)
    }

    pub fn blacklist<I>(&mut self, ips: I) -> &mut Self
    where
        I: IntoIpAddrs,
    {
        self.inner.set_blacklist(Some(ips));
        self
    }

    pub fn whitelist<I>(&mut self, ips: I) -> &mut Self
    where
        I: IntoIpAddrs,
    {
        self.inner.set_whitelist(Some(ips));
        self
    }

    pub fn plain_registry<I, E>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        self.inner.set_plain_registry(Some(iter));
        self
    }

    pub fn curve_registry<I, E>(&mut self, keys: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<CurvePublicKey>,
    {
        self.inner.set_curve_registry(Some(keys));
        self
    }

    pub fn no_curve_auth(&mut self) -> &mut Self {
        self.inner.set_curve_auth(Some(false));
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Client;

    use std::time::Duration;

    #[test]
    fn test_blacklist() {
        // Create a new context use a disctinct auth handler.
        let ctx = Ctx::new();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new()
            .bind(&addr)
            .recv_timeout(Duration::from_millis(200))
            .with_ctx(&ctx)
            .unwrap();

        // Blacklist the loopback addr.
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let _ = AuthBuilder::new().blacklist(ip).with_ctx(&ctx).unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client =
            ClientBuilder::new().connect(bound).with_ctx(&ctx).unwrap();

        client.try_send("").unwrap();
        assert!(server.recv_msg().is_err());
    }

    #[test]
    fn test_whitelist() {
        // Create a new context use a disctinct auth handler.
        let ctx = Ctx::new();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new()
            .bind(&addr)
            .recv_timeout(Duration::from_millis(200))
            .with_ctx(&ctx)
            .unwrap();

        // Whitelist the loopback addr.
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let _ = AuthBuilder::new().whitelist(ip).with_ctx(&ctx).unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client =
            ClientBuilder::new().connect(bound).with_ctx(&ctx).unwrap();

        client.try_send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_null() {
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new()
            .bind(&addr)
            .recv_timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client = Client::new().unwrap();

        client.connect(bound).unwrap();

        client.try_send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_plain_denied() {
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::PlainServer)
            .recv_timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = Client::new().unwrap();

        let creds = PlainClientCreds::new("user", "pwd");
        client.set_mechanism(Mechanism::PlainClient(creds)).unwrap();
        client.connect(bound).unwrap();

        client.try_send("").unwrap();
        assert!(server.recv_msg().is_err());
    }

    #[test]
    fn test_plain() {
        let ctx = Ctx::new();

        let creds = PlainClientCreds::new("user", "pwd");
        let _ = AuthBuilder::new()
            .plain_registry(&creds)
            .with_ctx(&ctx)
            .unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::PlainServer)
            .recv_timeout(Duration::from_millis(200))
            .with_ctx(&ctx)
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = ClientBuilder::new()
            .connect(bound)
            .mechanism(creds)
            .with_ctx(&ctx)
            .unwrap();

        client.try_send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_curve() {
        let ctx = Ctx::new();

        let server_cert = CurveCert::new_unique();
        let client_cert = CurveCert::new_unique();

        let _ = AuthBuilder::new()
            .curve_registry(client_cert.public())
            .with_ctx(&ctx)
            .unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server_creds = CurveServerCreds {
            secret: server_cert.secret().to_owned(),
        };

        let client_creds =
            CurveClientCreds::new(server_cert.public()).add_cert(client_cert);

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(server_creds)
            .recv_timeout(Duration::from_millis(200))
            .with_ctx(&ctx)
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = ClientBuilder::new()
            .mechanism(client_creds)
            .connect(bound)
            .with_ctx(&ctx)
            .unwrap();

        client.try_send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_curve_denied() {
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server_cert = CurveCert::new_unique();
        let server_creds = CurveServerCreds::new(server_cert.secret());

        let client_creds = CurveClientCreds::new(server_cert.public());

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(server_creds)
            .recv_timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = ClientBuilder::new()
            .mechanism(client_creds)
            .connect(bound)
            .build()
            .unwrap();

        client.try_send("").unwrap();
        assert!(server.recv_msg().is_err());
    }

    #[test]
    fn test_curve_no_auth() {
        let ctx = Ctx::new();

        let server_cert = CurveCert::new_unique();
        let server_creds = CurveServerCreds::new(server_cert.secret());

        let _ = AuthBuilder::new().no_curve_auth().with_ctx(&ctx).unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(server_creds)
            .recv_timeout(Duration::from_millis(200))
            .with_ctx(&ctx)
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = Client::with_ctx(&ctx).unwrap();

        let client_creds = CurveClientCreds::new(server_cert.public());
        client.set_mechanism(client_creds).unwrap();
        client.connect(bound).unwrap();

        client.try_send("").unwrap();
        server.recv_msg().unwrap();
    }
}
