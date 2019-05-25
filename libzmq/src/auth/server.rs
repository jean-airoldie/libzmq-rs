use super::*;
use crate::{addr::IntoIpAddrs, old::*, poll::*, prelude::*, socket::*, *};

use failure::Fail;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};

use libc::c_long;

use std::{
    convert::{TryFrom, TryInto},
    fmt,
    net::{IpAddr, Ipv6Addr},
    option, vec,
};

const ZAP_VERSION: &str = "1.0";
lazy_static! {
    static ref ZAP_ENDPOINT: InprocAddr = "zeromq.zap.01".try_into().unwrap();
    static ref COMMAND_ENDPOINT: InprocAddr = InprocAddr::new_unique();
    static ref AUTH_EVENT_ENDPOINT: InprocAddr = InprocAddr::new_unique();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlainClientCreds {
    pub username: String,
    pub password: String,
}

impl<'a> From<&'a PlainClientCreds> for PlainClientCreds {
    fn from(creds: &'a PlainClientCreds) -> Self {
        creds.to_owned()
    }
}

impl<'a> From<&'a PlainClientCreds> for Mechanism {
    fn from(creds: &'a PlainClientCreds) -> Self {
        Self::from(creds.to_owned())
    }
}

impl From<PlainClientCreds> for Mechanism {
    fn from(creds: PlainClientCreds) -> Self {
        Mechanism::PlainClient(creds)
    }
}

impl IntoIterator for PlainClientCreds {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a PlainClientCreds {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

/// Credentials for a `Curve` client.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveClientCreds {
    /// The client's `CurveCert`.
    pub client: CurveCert,
    /// The server's public `CurveKey`.
    ///
    /// This is used to ensure the server's identity.
    pub server: CurveKey,
}

impl<'a> From<&'a CurveClientCreds> for CurveClientCreds {
    fn from(creds: &'a CurveClientCreds) -> Self {
        creds.to_owned()
    }
}

impl<'a> From<&'a CurveClientCreds> for Mechanism {
    fn from(creds: &'a CurveClientCreds) -> Self {
        Mechanism::CurveClient(creds.to_owned())
    }
}

impl From<CurveClientCreds> for Mechanism {
    fn from(creds: CurveClientCreds) -> Self {
        Mechanism::CurveClient(creds)
    }
}

/// Credentials for a `Curve` server.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveServerCreds {
    /// The server's secret `CurveKey`.
    pub secret: CurveKey,
}

impl<'a> From<&'a CurveServerCreds> for CurveServerCreds {
    fn from(creds: &'a CurveServerCreds) -> Self {
        creds.to_owned()
    }
}

impl<'a> From<&'a CurveServerCreds> for Mechanism {
    fn from(creds: &'a CurveServerCreds) -> Self {
        Mechanism::CurveServer(creds.to_owned())
    }
}

impl From<CurveServerCreds> for Mechanism {
    fn from(creds: CurveServerCreds) -> Self {
        Mechanism::CurveServer(creds)
    }
}

/// A socket's `Mechanism`.
///
/// The `Mechanism` is used to configure the authentication and encryption
/// strategy to use between two connected sockets.
///
/// By default the `Null`
/// mechanism is used, meaning there is no attempt authentication nor encryption.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mechanism {
    /// No encryption or authentication.
    ///
    /// A socket using the `Null` mechanism can only connect other sockets
    /// using the same mechanism.
    Null,
    /// Plain text authentication with no encryption.
    ///
    /// A socket using the `PlainClient` mechanism can only connect to a
    /// socket using the `PlainServer`.
    PlainClient(PlainClientCreds),
    /// Plain text authentication with no encryption.
    ///
    /// A socket using the `PlainServer` mechanism can only connect to a
    /// socket using the `PlainClient`.
    PlainServer,
    /// Secure authentication and encryption using the `Curve` public-key mechanism.
    ///
    /// By default authentication is done using a whitelist of public keys.
    /// However, authentication can be disabled.
    ///
    /// A socket using the `CurveClient` mechanism can only connect to a
    /// socket using the `CurveServer`.
    CurveClient(CurveClientCreds),
    /// Secure authentication and encryption using the `Curve` public-key mechanism.
    ///
    /// A socket using the `CurveClient` mechanism can only connect to a
    /// socket using the `CurveServer`.
    CurveServer(CurveServerCreds),
}

impl<'a> From<&'a Mechanism> for Mechanism {
    fn from(mechanism: &'a Mechanism) -> Self {
        mechanism.to_owned()
    }
}

impl Default for Mechanism {
    fn default() -> Self {
        Mechanism::Null
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MechanismName {
    Null,
    Plain,
    Curve,
}

#[derive(Debug, Fail)]
#[fail(display = "unsupported mechanism")]
struct InvalidMechanismName;

impl<'a> TryFrom<&'a str> for MechanismName {
    type Error = InvalidMechanismName;

    fn try_from(s: &'a str) -> Result<MechanismName, InvalidMechanismName> {
        match s {
            "NULL" => Ok(MechanismName::Null),
            "PLAIN" => Ok(MechanismName::Plain),
            "CURVE" => Ok(MechanismName::Curve),
            _ => Err(InvalidMechanismName),
        }
    }
}

/// The possible status code resulting from a `ZAP` handshake.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StatusCode {
    Allowed = 200,
    TemporaryError = 300,
    Denied = 400,
    InternalError = 500,
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StatusCode::Allowed => write!(f, "{}", StatusCode::Allowed as i32),
            StatusCode::TemporaryError => {
                write!(f, "{}", StatusCode::TemporaryError as i32)
            }
            StatusCode::Denied => write!(f, "{}", StatusCode::Denied as i32),
            StatusCode::InternalError => {
                write!(f, "{}", StatusCode::InternalError as i32)
            }
        }
    }
}

#[derive(Debug, Fail)]
#[fail(display = "unable to parse status code")]
#[doc(hidden)]
pub struct StatusCodeParseError(());

impl TryFrom<c_long> for StatusCode {
    type Error = StatusCodeParseError;
    fn try_from(i: c_long) -> Result<Self, Self::Error> {
        match i {
            i if i == StatusCode::Allowed as c_long => Ok(StatusCode::Allowed),
            i if i == StatusCode::TemporaryError as c_long => {
                Ok(StatusCode::TemporaryError)
            }
            i if i == StatusCode::Denied as c_long => Ok(StatusCode::Denied),
            i if i == StatusCode::InternalError as c_long => {
                Ok(StatusCode::InternalError)
            }
            _ => Err(StatusCodeParseError(())),
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for StatusCode {
    type Error = StatusCodeParseError;
    fn try_from(a: &'a [u8]) -> Result<Self, Self::Error> {
        let mut bytes: [u8; 8] = Default::default();
        bytes.copy_from_slice(a);
        let code = c_long::from_ne_bytes(bytes);
        Self::try_from(code)
    }
}

#[derive(Clone, Debug)]
struct ZapRequest {
    version: String,
    request_id: Msg,
    domain: String,
    addr: Ipv6Addr,
    identity: Msg,
    mechanism: String,
    credentials: Vec<Msg>,
}

impl ZapRequest {
    fn new(mut parts: Vec<Msg>) -> Self {
        let version = parts.remove(0).to_str().unwrap().to_owned();
        assert_eq!(version, ZAP_VERSION);

        let request_id = parts.remove(0);
        let domain = parts.remove(0).to_str().unwrap().to_owned();
        let addr: Ipv6Addr = parts.remove(0).to_str().unwrap().parse().unwrap();

        let identity = parts.remove(0);

        let mechanism = parts.remove(0).to_str().unwrap().to_owned();

        Self {
            version,
            request_id,
            domain,
            addr,
            identity,
            mechanism,
            credentials: parts,
        }
    }
}

#[derive(Clone, Debug)]
struct ZapReply {
    version: String,
    request_id: Msg,
    status_code: StatusCode,
    status_text: String,
    user_id: String,
    metadata: Vec<u8>,
}

impl IntoIterator for ZapReply {
    type IntoIter = vec::IntoIter<Msg>;
    type Item = Msg;
    fn into_iter(self) -> Self::IntoIter {
        let parts: Vec<Msg> = vec![
            self.version.into(),
            self.request_id,
            self.status_code.to_string().into(),
            self.status_text.into(),
            self.user_id.into(),
            self.metadata.into(),
        ];

        parts.into_iter()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum AuthCommand {
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

struct AuthResult {
    user_id: String,
    metadata: Vec<u8>,
}

// A configurable ZAP handler.
pub(crate) struct AuthServer {
    //  ZAP handler socket
    handler: OldSocket,
    command: Server,
    whitelist: HashSet<Ipv6Addr>,
    blacklist: HashSet<Ipv6Addr>,
    plain_creds: HashMap<String, String>,
    curve_certs: HashSet<CurveKey>,
    curve_auth: bool,
}

impl AuthServer {
    pub(crate) fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx = ctx.into();
        let mut handler = OldSocket::with_ctx(OldSocketType::Router, &ctx)?;
        handler.bind(&*ZAP_ENDPOINT)?;

        let command = Server::with_ctx(ctx)?;
        command.bind(&*COMMAND_ENDPOINT).map_err(Error::cast)?;

        Ok(AuthServer {
            handler,
            command,
            whitelist: HashSet::default(),
            blacklist: HashSet::default(),
            plain_creds: HashMap::default(),
            curve_certs: HashSet::default(),
            curve_auth: true,
        })
    }

    pub(crate) fn run(&mut self) -> Result<(), Error> {
        let mut poller = Poller::new();
        poller.add(&self.handler, PollId(0), READABLE)?;
        poller.add(&self.command, PollId(1), READABLE)?;

        let mut events = Events::new();

        loop {
            poller.block(&mut events, None)?;

            for event in &events {
                if event.flags().contains(READABLE) {
                    match event.id() {
                        PollId(0) => {
                            let mut parts =
                                self.handler.recv_msg_multipart()?;
                            let routing_id = parts.remove(0);
                            assert!(parts.remove(0).is_empty());

                            let request = ZapRequest::new(parts);
                            let reply = self.on_zap(request)?;

                            self.handler.send(routing_id, true)?;
                            self.handler.send("", true)?;
                            self.handler.send_multipart(reply)?;
                        }
                        PollId(1) => {
                            let msg = self.command.recv_msg()?;
                            let id = msg.routing_id().unwrap();
                            let command: AuthCommand =
                                bincode::deserialize(msg.as_bytes()).unwrap();

                            self.on_command(command);
                            let mut msg = Msg::new();
                            msg.set_routing_id(id);
                            self.command.send(msg).map_err(Error::cast)?;
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    fn on_command(&mut self, command: AuthCommand) {
        match command {
            AuthCommand::AddWhitelist(ip) => {
                info!("added IP : {} to whitelist", &ip);
                self.whitelist.insert(ip);
            }
            AuthCommand::RemoveWhitelist(ip) => {
                info!("remove IP : {} to whitelist", &ip);
                self.whitelist.remove(&ip);
            }
            AuthCommand::AddBlacklist(ip) => {
                info!("added IP : {} to blacklist", &ip);
                self.blacklist.insert(ip);
            }
            AuthCommand::RemoveBlacklist(ip) => {
                info!("removed IP : {} from blacklist", &ip);
                self.blacklist.remove(&ip);
            }
            AuthCommand::AddPlainClientCreds(creds) => {
                info!("added user : {}", &creds.username);
                self.plain_creds.insert(creds.username, creds.password);
            }
            AuthCommand::RemovePlainClientCreds(username) => {
                info!("removed user: {}", &username);
                self.plain_creds.remove(&username);
            }
            AuthCommand::AddCurveKey(key) => {
                info!("added z85 key: {}", key.as_str());
                self.curve_certs.insert(key);
            }
            AuthCommand::RemoveCurveKey(key) => {
                info!("removed z85 key: {}", key.as_str());
                self.curve_certs.remove(&key);
            }
            AuthCommand::SetCurveAuth(enabled) => {
                if enabled {
                    info!("enabled curve auth");
                } else {
                    info!("disabled curve auth");
                }
                self.curve_auth = enabled;
            }
        }
    }

    fn on_zap(&mut self, mut request: ZapRequest) -> Result<ZapReply, Error> {
        let denied = {
            if !self.whitelist.is_empty()
                && !self.whitelist.contains(&request.addr)
            {
                info!("denied addr {}, not whitelisted", &request.addr);
                true
            } else if self.whitelist.is_empty()
                && !self.blacklist.is_empty()
                && self.blacklist.contains(&request.addr)
            {
                info!("denied addr {}, blacklisted", &request.addr);
                true
            } else {
                false
            }
        };

        let mut result = None;

        if !denied {
            if let Ok(mechanism) =
                MechanismName::try_from(request.mechanism.as_str())
            {
                result = {
                    match mechanism {
                        MechanismName::Null => Some(AuthResult {
                            user_id: String::new(),
                            metadata: vec![],
                        }),
                        MechanismName::Plain => {
                            let username = request
                                .credentials
                                .remove(0)
                                .to_str()
                                .unwrap()
                                .to_owned();

                            let password = request
                                .credentials
                                .remove(0)
                                .to_str()
                                .unwrap()
                                .to_owned();

                            let creds = PlainClientCreds { username, password };
                            self.auth_plain(creds)
                        }
                        MechanismName::Curve => {
                            let curve_public_key = BinCurveKey::new_unchecked(
                                request
                                    .credentials
                                    .remove(0)
                                    .as_bytes()
                                    .to_owned(),
                            );
                            let z85_public_key: CurveKey =
                                curve_public_key.into();

                            self.auth_curve(z85_public_key)
                        }
                    }
                };
            }
        }

        if let Some(result) = result {
            Ok(ZapReply {
                request_id: request.request_id,
                user_id: result.user_id,
                metadata: result.metadata,
                version: ZAP_VERSION.to_owned(),
                status_code: StatusCode::Allowed,
                status_text: "OK".to_owned(),
            })
        } else {
            Ok(ZapReply {
                request_id: request.request_id,
                user_id: String::new(),
                metadata: vec![],
                version: ZAP_VERSION.to_owned(),
                status_code: StatusCode::Denied,
                status_text: "NOT OK".to_owned(),
            })
        }
    }

    fn auth_plain(&mut self, creds: PlainClientCreds) -> Option<AuthResult> {
        match self.plain_creds.get(&creds.username) {
            Some(password) => {
                info!("allowed user: {}", &creds.username);
                if password == &creds.password {
                    Some(AuthResult {
                        user_id: creds.username,
                        metadata: vec![],
                    })
                } else {
                    None
                }
            }
            None => {
                info!("denied user: {}", &creds.username);
                None
            }
        }
    }

    fn auth_curve(&mut self, public_key: CurveKey) -> Option<AuthResult> {
        if !self.curve_auth {
            info!("allowed curve public key {}", public_key);
            Some(AuthResult {
                user_id: String::new(),
                metadata: vec![],
            })
        } else if self.curve_certs.contains(&public_key) {
            info!("allowed curve public key {}", public_key);
            Some(AuthResult {
                user_id: public_key.as_str().to_owned(),
                metadata: vec![],
            })
        } else {
            info!("denied curve public key {}", public_key);
            None
        }
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
