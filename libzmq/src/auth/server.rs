use super::{client::*, *};
use crate::{old::*, poll::*, prelude::*, socket::*, *};

use failure::Fail;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use log::info;

use libc::c_long;

use std::{fmt, net::Ipv6Addr, vec};

const ZAP_VERSION: &str = "1.0";
lazy_static! {
    static ref ZAP_ENDPOINT: InprocAddr = "zeromq.zap.01".try_into().unwrap();
    pub(crate) static ref COMMAND_ENDPOINT: InprocAddr =
        InprocAddr::new_unique();
    static ref AUTH_EVENT_ENDPOINT: InprocAddr = InprocAddr::new_unique();
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

struct AuthResult {
    user_id: String,
    metadata: Vec<u8>,
}

// A configurable ZAP handler.
pub(crate) struct AuthServer {
    //  ZAP handler socket
    handler: OldSocket,
    request: Server,
    whitelist: HashSet<Ipv6Addr>,
    blacklist: HashSet<Ipv6Addr>,
    plain_registry: HashMap<String, String>,
    // Allowed public client keys.
    curve_registry: HashSet<CurvePublicKey>,
    // Whether curve auth is enabled.
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

        let request = Server::with_ctx(ctx)?;
        request.bind(&*COMMAND_ENDPOINT).map_err(Error::cast)?;

        Ok(AuthServer {
            handler,
            request,
            whitelist: HashSet::default(),
            blacklist: HashSet::default(),
            plain_registry: HashMap::default(),
            curve_registry: HashSet::default(),
            curve_auth: true,
        })
    }

    pub(crate) fn run(&mut self) -> Result<(), Error> {
        let mut poller = Poller::new();
        poller.add(&self.handler, PollId(0), READABLE)?;
        poller.add(&self.request, PollId(1), READABLE)?;

        let mut events = Events::new();

        loop {
            poller.poll(&mut events, Period::Infinite)?;

            for event in &events {
                match event.id() {
                    PollId(0) => {
                        let mut parts = self.handler.recv_msg_multipart()?;
                        let routing_id = parts.remove(0);
                        assert!(parts.remove(0).is_empty());

                        let request = ZapRequest::new(parts);
                        let reply = self.on_zap(request)?;

                        self.handler.send(routing_id, true)?;
                        self.handler.send("", true)?;
                        self.handler.send_multipart(reply)?;
                    }
                    PollId(1) => {
                        let msg = self.request.recv_msg()?;
                        let id = msg.routing_id().unwrap();
                        let request: AuthRequest =
                            bincode::deserialize(msg.as_bytes()).unwrap();

                        let reply = self.on_request(request);
                        let ser = bincode::serialize(&reply).unwrap();

                        self.request.route(ser, id).map_err(Error::cast)?;
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn on_request(&mut self, request: AuthRequest) -> AuthReply {
        match request {
            AuthRequest::AddWhitelist(ip) => {
                info!("added IP : {} to whitelist", &ip);
                self.whitelist.insert(ip);

                AuthReply::Success
            }
            AuthRequest::RemoveWhitelist(ip) => {
                info!("remove IP : {} to whitelist", &ip);
                self.whitelist.remove(&ip);

                AuthReply::Success
            }
            AuthRequest::SetWhitelist(ips) => {
                info!("reset whitelist");
                self.whitelist.clear();
                info!("added IPs: {:#?} to whitelist", &ips);
                self.whitelist.extend(ips);

                AuthReply::Success
            }
            AuthRequest::AddBlacklist(ip) => {
                info!("added IP : {} to blacklist", &ip);
                self.blacklist.insert(ip);

                AuthReply::Success
            }
            AuthRequest::RemoveBlacklist(ip) => {
                info!("removed IP : {} from blacklist", &ip);
                self.blacklist.remove(&ip);

                AuthReply::Success
            }
            AuthRequest::SetBlacklist(ips) => {
                info!("reset blacklist");
                self.blacklist.clear();
                info!("added IPs: {:#?} to blacklist", &ips);
                self.blacklist.extend(ips);

                AuthReply::Success
            }
            AuthRequest::AddPlainRegistry(creds) => {
                info!("added user : {} to plain registry", &creds.username);
                self.plain_registry.insert(creds.username, creds.password);

                AuthReply::Success
            }
            AuthRequest::RemovePlainRegistry(username) => {
                info!("removed user: {} from plain registry", &username);
                self.plain_registry.remove(&username);

                AuthReply::Success
            }
            AuthRequest::SetPlainRegistry(creds) => {
                info!("reset plain registry");
                self.plain_registry.clear();
                let users: Vec<&str> =
                    creds.iter().map(|c| c.username.as_str()).collect();
                info!("added users : {:#?} to plain registry", users);
                self.plain_registry.extend(
                    creds.into_iter().map(|c| (c.username, c.password)),
                );

                AuthReply::Success
            }
            AuthRequest::AddCurveRegistry(key) => {
                info!("added public key: {} to curve registry", key.as_str());
                self.curve_registry.insert(key);

                AuthReply::Success
            }
            AuthRequest::RemoveCurveRegistry(key) => {
                info!("removed public key: {} to curve registry", key.as_str());
                self.curve_registry.remove(&key);

                AuthReply::Success
            }
            AuthRequest::SetCurveRegistry(keys) => {
                info!("reset cerve registry");
                self.curve_registry.clear();
                info!("added public keys: {:#?} to curve registry", &keys);
                self.curve_registry.extend(keys);

                AuthReply::Success
            }
            AuthRequest::SetCurveAuth(enabled) => {
                if enabled {
                    info!("enabled curve auth");
                } else {
                    info!("disabled curve auth");
                }
                self.curve_auth = enabled;

                AuthReply::Success
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
                            let bin_public_key = BinCurveKey::new_unchecked(
                                request
                                    .credentials
                                    .remove(0)
                                    .as_bytes()
                                    .to_owned(),
                            );
                            let public_key: CurvePublicKey =
                                bin_public_key.into();

                            self.auth_curve(public_key)
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
        match self.plain_registry.get(&creds.username) {
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

    fn auth_curve(&mut self, public_key: CurvePublicKey) -> Option<AuthResult> {
        if !self.curve_auth {
            info!("allowed curve public key {}", public_key);
            Some(AuthResult {
                user_id: String::new(),
                metadata: vec![],
            })
        } else if self.curve_registry.contains(&public_key) {
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
