//! Socket authentication and encryption.

use crate::{old::*, poll::*, prelude::*, socket::*, *};

use failure::Fail;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};

use libc::c_long;

use std::{
    convert::{TryFrom, TryInto},
    fmt,
    net::IpAddr,
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
pub enum Mechanism {
    /// No encryption or authentication.
    ///
    /// This is the default mechanism.
    Null,
    /// Plain text authentication with no encryption.
    PlainClient(PlainCreds),
    /// Plain text authentication with no encryption.
    PlainServer,
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
    Null = 0,
    Plain,
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
        let code = dbg!(c_long::from_ne_bytes(bytes));
        Self::try_from(code)
    }
}

#[derive(Clone, Debug)]
struct ZapRequest {
    version: String,
    request_id: Msg,
    domain: String,
    addr: IpAddr,
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
        let addr: IpAddr = parts.remove(0).to_str().unwrap().parse().unwrap();

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
enum Command {}

pub struct Blacklist<'a> {
    inner: &'a Client,
}

impl<'a> Blacklist<'a> {
    pub fn insert(_ip: IpAddr) -> Result<(), Error> {
        unimplemented!()
    }

    pub fn remove(_ip: IpAddr) -> Result<(), Error> {
        unimplemented!()
    }
}

pub struct Whitelist<'a> {
    inner: &'a Client,
}

impl<'a> Whitelist<'a> {
    pub fn insert(_ip: IpAddr) -> Result<(), Error> {
        unimplemented!()
    }

    pub fn remove(_ip: IpAddr) -> Result<(), Error> {
        unimplemented!()
    }
}

pub struct PlainRegistry<'a> {
    inner: &'a Client,
}

impl<'a> PlainRegistry<'a> {
    pub fn insert(_creds: PlainCreds) -> Result<(), Error> {
        unimplemented!()
    }

    pub fn remove(_username: String) -> Result<(), Error> {
        unimplemented!()
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
    pub fn blacklist(&self) -> Blacklist {
        Blacklist {
            inner: &self.client,
        }
    }

    pub fn whitelist(&self) -> Whitelist {
        Whitelist {
            inner: &self.client,
        }
    }

    pub fn plain(&self) -> PlainRegistry {
        PlainRegistry {
            inner: &self.client,
        }
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
    whitelist: HashSet<IpAddr>,
    blacklist: HashSet<IpAddr>,
    passwords: HashMap<String, String>,
    proxy: bool,
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
            proxy: false,
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
                            let _msg = self.command.recv_msg()?;
                            unimplemented!();
                        }
                        _ => unreachable!(),
                    }
                }
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
            } else if !self.blacklist.is_empty()
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
            if self.proxy {
                unimplemented!()
            } else if let Ok(mechanism) =
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{auth::*, monitor::*, prelude::*, socket::*};

    use std::convert::TryInto;

    fn expect_event(monitor: &mut SocketMonitor, expected: EventType) {
        let event = monitor.recv_event().unwrap();
        assert_eq!(event.event_type(), expected);
    }

    #[test]
    fn test_null_mechanism() {
        let mut monitor = SocketMonitor::new().unwrap();
        monitor.subscribe(EventCode::HandshakeSucceeded).unwrap();

        let addr: TcpAddr = "127.0.0.1:*".try_into().unwrap();
        let server = ServerBuilder::new().bind(&addr).build().unwrap();

        let bound = server.last_endpoint().unwrap().unwrap();
        let client = ClientBuilder::new().connect(bound).build().unwrap();

        monitor.register(&server).unwrap();
        monitor.register(&client).unwrap();

        expect_event(&mut monitor, EventType::HandshakeSucceeded);
        expect_event(&mut monitor, EventType::HandshakeSucceeded);
    }

    #[test]
    fn test_plain_mechanism_invalid_creds() {
        let mut monitor = SocketMonitor::new().unwrap();
        monitor.subscribe(EventCode::HandshakeSucceeded).unwrap();
        monitor.subscribe(EventCode::HandshakeFailedAuth).unwrap();
        monitor
            .subscribe(EventCode::HandshakeFailedNoDetail)
            .unwrap();
        monitor
            .subscribe(EventCode::HandshakeFailedProtocol)
            .unwrap();

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

        let client = ClientBuilder::new()
            .connect(bound)
            .mechanism(Mechanism::PlainClient(creds))
            .build()
            .unwrap();

        monitor.register(&server).unwrap();
        monitor.register(&client).unwrap();

        expect_event(
            &mut monitor,
            EventType::HandshakeFailedAuth(StatusCode::Denied),
        );
        expect_event(
            &mut monitor,
            EventType::HandshakeFailedAuth(StatusCode::Denied),
        );
    }
}
