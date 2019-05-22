//! Socket authentication and encryption.

pub mod curve;

use crate::{old::*, poll::*, prelude::*, socket::*, *};
use curve::*;

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
    time::Duration,
    vec,
};

const ZAP_VERSION: &str = "1.0";
lazy_static! {
    static ref ZAP_ENDPOINT: InprocAddr = "zeromq.zap.01".try_into().unwrap();
    static ref COMMAND_ENDPOINT: InprocAddr = InprocAddr::new_unique();
    static ref AUTH_EVENT_ENDPOINT: InprocAddr = InprocAddr::new_unique();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlainCreds {
    pub username: String,
    pub password: String,
}

impl Into<Mechanism> for PlainCreds {
    fn into(self) -> Mechanism {
        Mechanism::PlainClient(self)
    }
}

impl<'a> Into<Mechanism> for &'a PlainCreds {
    fn into(self) -> Mechanism {
        self.to_owned().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveClientCreds {
    /// The client's z85 key pair.
    pub client: Z85Cert,
    /// The server's public key.
    pub server: Z85Key,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveServerCreds {
    /// The server's secret key.
    pub secret: Z85Key,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mechanism {
    /// No encryption or authentication.
    ///
    /// This is the default mechanism.
    Null,
    /// Plain text authentication with no encryption.
    PlainClient(PlainCreds),
    /// Plain text authentication with no encryption.
    PlainServer,
    CurveClient(CurveClientCreds),
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
            _ => Err(InvalidMechanismName),
        }
    }
}

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
    version: String, //  Version number, must be "1.0"
    request_id: Msg, //  Sequence number of request
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
enum Command {
    AddBlacklist(Ipv6Addr),
    RemoveBlacklist(Ipv6Addr),
    AddWhitelist(Ipv6Addr),
    RemoveWhitelist(Ipv6Addr),
    AddCurveCert(Z85Key),
    RemoveCurveCert(Z85Key),
}

fn into_ipv6(ip: IpAddr) -> Ipv6Addr {
    match ip {
        IpAddr::V4(ipv4) => ipv4.to_ipv6_mapped(),
        IpAddr::V6(ipv6) => ipv6,
    }
}

/// A communication channel to the `AuthHandler`.
///
/// This allows for seemless thread safe state updates to the `AuthHandler` via
/// RPC communication.
pub struct AuthChannel {
    client: Client,
}

impl AuthChannel {
    pub fn new() -> Result<Self, Error> {
        Self::with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let client = ClientBuilder::new()
            .connect(&*COMMAND_ENDPOINT)
            .with_ctx(ctx)
            .map_err(Error::cast)?;

        Ok(Self { client })
    }

    fn command(&self, command: &Command) -> Result<(), Error> {
        let ser = bincode::serialize(command).unwrap();

        self.client.send(ser).map_err(Error::cast)?;
        let msg = self.client.recv_msg()?;
        assert!(msg.is_empty());
        Ok(())
    }

    pub fn add_blacklist(&self, ip: IpAddr) -> Result<(), Error> {
        let ipv6 = into_ipv6(ip);
        self.command(&Command::AddBlacklist(ipv6))
    }

    pub fn remove_blacklist(&self, ip: IpAddr) -> Result<(), Error> {
        let ipv6 = into_ipv6(ip);
        self.command(&Command::RemoveBlacklist(ipv6))
    }

    pub fn add_whitelist(&self, ip: IpAddr) -> Result<(), Error> {
        let ipv6 = into_ipv6(ip);
        self.command(&Command::AddWhitelist(ipv6))
    }

    pub fn remove_whitelist(&self, ip: IpAddr) -> Result<(), Error> {
        let ipv6 = into_ipv6(ip);
        self.command(&Command::RemoveWhitelist(ipv6))
    }

    pub fn add_curve_cert(&self, key: Z85Key) -> Result<(), Error> {
        self.command(&Command::AddCurveCert(key))
    }

    pub fn remove_curve_cert(&self, key: Z85Key) -> Result<(), Error> {
        self.command(&Command::RemoveCurveCert(key))
    }
}

struct AuthResult {
    user_id: String,
    metadata: Vec<u8>,
}

pub(crate) struct AuthHandler {
    //  ZAP handler socket
    handler: OldSocket,
    command: Server,
    whitelist: HashSet<Ipv6Addr>,
    blacklist: HashSet<Ipv6Addr>,
    passwords: HashMap<String, String>,
    curve_no_auth: bool,
}

impl AuthHandler {
    pub(crate) fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx = ctx.into();
        let mut handler = OldSocket::with_ctx(OldSocketType::Router, &ctx)?;
        handler.bind(&*ZAP_ENDPOINT)?;

        let command = Server::with_ctx(ctx)?;
        command.bind(&*COMMAND_ENDPOINT).map_err(Error::cast)?;

        Ok(Self {
            handler,
            command,
            whitelist: HashSet::default(),
            blacklist: HashSet::default(),
            passwords: HashMap::default(),
            curve_no_auth: false,
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
                            let command: Command =
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

    fn on_command(&mut self, command: Command) {
        match command {
            Command::AddWhitelist(ip) => {
                info!("added IP : {} to whitelist", &ip);
                self.whitelist.insert(ip);
            }
            Command::RemoveWhitelist(ip) => {
                info!("remove IP : {} to whitelist", &ip);
                self.whitelist.remove(&ip);
            }
            Command::AddBlacklist(ip) => {
                info!("added IP : {} to blacklist", &ip);
                self.blacklist.insert(ip);
            }
            Command::RemoveBlacklist(ip) => {
                info!("removed IP : {} from blacklist", &ip);
                self.blacklist.remove(&ip);
            }
            Command::AddCurveCert(key) => unimplemented!(),
            Command::RemoveCurveCert(key) => unimplemented!(),
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

                            let creds = PlainCreds { username, password };
                            self.auth_plain(creds)
                        }
                        MechanismName::Curve => {
                            let curve_public_key = CurveKey::new_unchecked(
                                request
                                    .credentials
                                    .remove(0)
                                    .as_bytes()
                                    .to_owned(),
                            );

                            let z85_public_key: Z85Key =
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

    fn auth_plain(&mut self, creds: PlainCreds) -> Option<AuthResult> {
        if self.passwords.is_empty() {
            None
        } else {
            match self.passwords.get(&creds.username) {
                Some(password) => {
                    if password == &creds.password {
                        Some(AuthResult {
                            user_id: creds.username,
                            metadata: vec![],
                        })
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
    }

    fn auth_curve(&mut self, public_key: Z85Key) -> Option<AuthResult> {
        if self.curve_no_auth {
            Some(AuthResult {
                user_id: String::new(),
                metadata: vec![],
            })
        } else {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{auth::*, monitor::*, prelude::*, socket::*, Client};

    use std::convert::TryInto;

    fn expect_event(monitor: &mut SocketMonitor, expected: EventType) {
        let event = monitor.recv_event().unwrap();
        assert_eq!(event.event_type(), expected);
    }

    // This test might fail see:
    // https://github.com/zeromq/libzmq/issues/3519
    // https://github.com/jean-airoldie/libzmq-rs/issues/30
    #[test]
    fn test_blacklist() {
        // Create a new context use a disctinct auth handler.
        let ctx = Ctx::new();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new()
            .bind(&addr)
            .recv_timeout(Duration::from_millis(300))
            .with_ctx(&ctx)
            .unwrap();

        // Blacklist the loopback addr.
        let channel = AuthChannel::with_ctx(&ctx).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        channel.add_blacklist(ip).unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client =
            ClientBuilder::new().connect(bound).with_ctx(&ctx).unwrap();

        client.send("").unwrap();
        assert!(server.recv_msg().is_err());

        channel.remove_blacklist(ip).unwrap();
        server.recv_msg().unwrap();
    }

    // This test might fail see:
    // https://github.com/zeromq/libzmq/issues/3519
    // https://github.com/jean-airoldie/libzmq-rs/issues/30
    #[test]
    fn test_whitelist() {
        // Create a new context use a disctinct auth handler.
        let ctx = Ctx::new();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new()
            .bind(&addr)
            .recv_timeout(Duration::from_millis(300))
            .with_ctx(&ctx)
            .unwrap();

        // Blacklist the loopback addr.
        let channel = AuthChannel::with_ctx(&ctx).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        channel.add_whitelist(ip).unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client =
            ClientBuilder::new().connect(bound).with_ctx(&ctx).unwrap();

        client.send("").unwrap();
        server.recv_msg().unwrap();
    }

    #[test]
    fn test_null_mechanism() {
        let mut monitor = SocketMonitor::new().unwrap();
        monitor.subscribe(EventCode::HandshakeSucceeded).unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new().bind(&addr).build().unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client = Client::new().unwrap();

        monitor.register(&client).unwrap();

        client.connect(bound).unwrap();

        expect_event(&mut monitor, EventType::HandshakeSucceeded);
    }

    #[test]
    fn test_plain_mechanism_invalid_creds() {
        let mut monitor = SocketMonitor::new().unwrap();
        monitor.subscribe(EventCode::HandshakeFailedAuth).unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::PlainServer)
            .build()
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let username = "ok".to_owned();
        let password = "lo".to_owned();

        let creds = PlainCreds { username, password };

        let client = Client::new().unwrap();

        monitor.register(&client).unwrap();

        client.set_mechanism(Mechanism::PlainClient(creds)).unwrap();
        client.connect(bound).unwrap();

        expect_event(
            &mut monitor,
            EventType::HandshakeFailedAuth(StatusCode::Denied),
        );
    }

    #[test]
    fn test_curve_mechanism_invalid_creds() {
        let mut monitor = SocketMonitor::new().unwrap();
        monitor.subscribe(EventCode::HandshakeFailedAuth).unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();

        let server_cert = Z85Cert::new_unique();
        let client_cert = Z85Cert::new_unique();

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
            .build()
            .unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();

        let client = Client::new().unwrap();

        monitor.register(&client).unwrap();

        client.set_mechanism(client_creds).unwrap();
        client.connect(bound).unwrap();

        expect_event(
            &mut monitor,
            EventType::HandshakeFailedAuth(StatusCode::Denied),
        );
    }
}
