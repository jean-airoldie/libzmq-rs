use super::{server::COMMAND_ENDPOINT, *};
use crate::{addr::IntoIpAddrs, prelude::*, socket::*, *};

use serde::{Deserialize, Serialize};

use std::net::{IpAddr, Ipv6Addr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum AuthCommand {
    AddBlacklist(Ipv6Addr),
    RemoveBlacklist(Ipv6Addr),
    AddWhitelist(Ipv6Addr),
    RemoveWhitelist(Ipv6Addr),
    AddPlainClientCreds(PlainClientCreds),
    RemovePlainClientCreds(String),
    AddCurveKey(CurveKey),
    RemoveCurveKey(CurveKey),
    SetCurveAuth(bool),
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
/// use std::{time::Duration, convert::TryInto};
///
/// let server_cert = CurveCert::new_unique();
/// let client_cert = CurveCert::new_unique();
///
/// let addr: TcpAddr = "127.0.0.1:*".try_into()?;
///
/// let server_creds = CurveServerCreds {
///     secret: server_cert.secret().to_owned(),
/// };
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
/// let _ = AuthBuilder::new().curve_keys(client_cert.public()).build()?;
///
/// let bound = server.last_endpoint()?;
///
/// let client_creds = CurveClientCreds {
///     client: client_cert,
///     server: server_cert.public().to_owned(),
/// };
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

    fn command(&self, command: &AuthCommand) -> Result<(), Error> {
        let ser = bincode::serialize(command).unwrap();

        self.client.send(ser).map_err(Error::cast)?;
        let msg = self.client.recv_msg()?;
        assert!(msg.is_empty());
        Ok(())
    }

    /// Add the ip to the `AuthServer`'s blacklist.
    ///
    /// Blacklisted ips will be denied access.
    pub fn add_blacklist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ip in ips.into_ip_addrs() {
            let ipv6 = into_ipv6(ip);
            self.command(&AuthCommand::AddBlacklist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the ip from the `AuthServer`'s blacklist, if it
    /// is present.
    pub fn remove_blacklist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ip in ips.into_ip_addrs() {
            let ipv6 = into_ipv6(ip);
            self.command(&AuthCommand::RemoveBlacklist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Add the ip to the `AuthServer`'s whitelist.
    ///
    /// If the whitelist is not empty, only ips in present
    /// in the whitelist are allowed. The whitelist takes precedence
    /// over the blacklist.
    pub fn add_whitelist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ip in ips.into_ip_addrs() {
            let ipv6 = into_ipv6(ip);
            self.command(&AuthCommand::AddWhitelist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the ip from the `AuthServer`'s whitelist, if it
    /// is present.
    pub fn remove_whitelist<I>(&self, ips: I) -> Result<(), Error<usize>>
    where
        I: IntoIpAddrs,
    {
        let mut count = 0;

        for ip in ips.into_ip_addrs() {
            let ipv6 = into_ipv6(ip);
            self.command(&AuthCommand::RemoveWhitelist(ipv6))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Add the plain client's credentials to the `AuthServer`'s whitelist.
    ///
    /// Only credentials present in the whitelist can successfully authenticate.
    pub fn add_plain_creds<I, E>(&self, iter: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        let mut count = 0;

        for creds in iter.into_iter() {
            let creds = creds.into();
            self.command(&AuthCommand::AddPlainClientCreds(creds))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the username's credentials from the `AuthServer`'s whitelist
    /// if they exist.
    pub fn remove_plain_creds<I, E>(
        &self,
        usernames: I,
    ) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<String>,
    {
        let mut count = 0;

        for username in usernames.into_iter() {
            let username = username.into();
            self.command(&AuthCommand::RemovePlainClientCreds(username))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Add the given public `CurveKey` to the whitelist.
    ///
    /// Only public keys present in the whitelist are allowed to authenticate
    /// via the `CURVE` mechanism.
    pub fn add_curve_keys<I, E>(&self, keys: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<CurveKey>,
    {
        let mut count = 0;

        for key in keys.into_iter() {
            let key = key.into();
            self.command(&AuthCommand::AddCurveKey(key))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Remove the given public `CurveKey` from the `AuthServer`'s store
    /// if it is present.
    pub fn remove_curve_keys<I, E>(&self, keys: I) -> Result<(), Error<usize>>
    where
        I: IntoIterator<Item = E>,
        E: Into<CurveKey>,
    {
        let mut count = 0;

        for key in keys.into_iter() {
            let key = key.into();
            self.command(&AuthCommand::RemoveCurveKey(key))
                .map_err(|err| Error::with_content(err.kind(), count))?;

            count += 1;
        }
        Ok(())
    }

    /// Sets whether to use authentication for the `CURVE` mechanism.
    ///
    /// If it is set to `true`, then only sockets whose public key is present
    /// in the whitelist will be allowed to authenticate. Otherwise all sockets
    /// authenticate successfully.
    pub fn set_curve_auth(&self, enabled: bool) -> Result<(), Error> {
        self.command(&AuthCommand::SetCurveAuth(enabled))
    }
}

/// A Configuration of the `AuthServer`.
///
/// A `AuthClient` must be used to communicate
/// the configuration with the server.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthConfig {
    blacklist: Option<Vec<IpAddr>>,
    whitelist: Option<Vec<IpAddr>>,
    plain_creds: Option<Vec<PlainClientCreds>>,
    curve_keys: Option<Vec<CurveKey>>,
    curve_auth: Option<bool>,
}

impl AuthConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<AuthClient, Error<usize>> {
        self.with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<AuthClient, Error<usize>>
    where
        C: Into<Ctx>,
    {
        let ctx: Ctx = ctx.into();
        let client = AuthClient::with_ctx(ctx).map_err(Error::cast)?;
        self.apply(&client)?;

        Ok(client)
    }

    pub fn apply(&self, client: &AuthClient) -> Result<(), Error<usize>> {
        if let Some(ref blacklist) = self.blacklist {
            client.add_blacklist(blacklist)?;
        }
        if let Some(ref whitelist) = self.whitelist {
            client.add_whitelist(whitelist)?;
        }
        if let Some(ref creds) = self.plain_creds {
            client.add_plain_creds(creds)?;
        }
        if let Some(ref keys) = self.curve_keys {
            client.add_curve_keys(keys)?;
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

    pub fn set_plain_creds<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        let maybe: Option<Vec<PlainClientCreds>> =
            maybe.map(|e| e.into_iter().map(Into::into).collect());
        self.plain_creds = maybe;
    }

    pub fn set_curve_keys<I, E>(&mut self, maybe: Option<I>)
    where
        I: IntoIterator<Item = E>,
        E: Into<CurveKey>,
    {
        let maybe: Option<Vec<CurveKey>> =
            maybe.map(|e| e.into_iter().map(Into::into).collect());
        self.curve_keys = maybe;
    }

    pub fn set_curve_auth(&mut self, maybe: Option<bool>) {
        self.curve_auth = maybe;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuthBuilder {
    inner: AuthConfig,
}

impl AuthBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Result<AuthClient, Error<usize>> {
        self.inner.build()
    }

    pub fn with_ctx<C>(&self, ctx: C) -> Result<AuthClient, Error<usize>>
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

    pub fn plain_creds<I, E>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<PlainClientCreds>,
    {
        self.inner.set_plain_creds(Some(iter));
        self
    }

    pub fn curve_keys<I, E>(&mut self, keys: I) -> &mut Self
    where
        I: IntoIterator<Item = E>,
        E: Into<CurveKey>,
    {
        self.inner.set_curve_keys(Some(keys));
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

    use std::{convert::TryInto, time::Duration};

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

        client.send("").unwrap();
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

        client.send("").unwrap();
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
        client.send("").unwrap();
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

        let username = "ok".to_owned();
        let password = "lo".to_owned();

        let creds = PlainClientCreds { username, password };

        let client = Client::new().unwrap();

        client.set_mechanism(Mechanism::PlainClient(creds)).unwrap();
        client.connect(bound).unwrap();

        client.send("").unwrap();
        assert!(server.recv_msg().is_err());
    }

    #[test]
    fn test_plain() {
        let ctx = Ctx::new();

        let username = "ok".to_owned();
        let password = "lo".to_owned();

        let creds = PlainClientCreds { username, password };
        let _ = AuthBuilder::new()
            .plain_creds(&creds)
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

        client.send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_curve() {
        let ctx = Ctx::new();

        let server_cert = CurveCert::new_unique();
        let client_cert = CurveCert::new_unique();

        let _ = AuthBuilder::new()
            .curve_keys(client_cert.public())
            .with_ctx(&ctx)
            .unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server_creds = CurveServerCreds {
            secret: server_cert.secret().to_owned(),
        };

        let client_creds = CurveClientCreds {
            client: client_cert,
            server: server_cert.public().to_owned(),
        };

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

        client.send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_curve_denied() {
        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server_cert = CurveCert::new_unique();
        let client_cert = CurveCert::new_unique();

        let server_creds = CurveServerCreds {
            secret: server_cert.secret().clone(),
        };

        let client_creds = CurveClientCreds {
            client: client_cert,
            server: server_cert.public().clone(),
        };

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

        client.send("").unwrap();
        assert!(server.recv_msg().is_err());
    }

    #[test]
    fn test_curve_no_auth() {
        let ctx = Ctx::new();

        let server_cert = CurveCert::new_unique();
        let client_cert = CurveCert::new_unique();

        let _ = AuthBuilder::new().no_curve_auth().with_ctx(&ctx).unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server_creds = CurveServerCreds {
            secret: server_cert.secret().to_owned(),
        };

        let client_creds = CurveClientCreds {
            client: client_cert,
            server: server_cert.public().to_owned(),
        };

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(server_creds)
            .recv_timeout(Duration::from_millis(200))
            .with_ctx(&ctx)
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = Client::with_ctx(&ctx).unwrap();

        client.set_mechanism(client_creds).unwrap();
        client.connect(bound).unwrap();

        client.send("").unwrap();
        server.recv_msg().unwrap();
    }
}
