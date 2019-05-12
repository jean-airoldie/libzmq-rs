mod dealer;

pub(crate) use dealer::Dealer;

use crate::{poll::*, *};

use failure::Fail;
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use std::{convert::TryFrom, fmt, net::IpAddr};

const ZAP_VERSION: &str = "1.0";
lazy_static! {
    static ref ZAP_ENDPOINT: InprocAddr = "zeromq.zap.01".parse().unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlainCreds {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mechanism {
    /// No encryption or authentication.
    ///
    /// This is the default mechanism.
    Null,
    PlainClient(PlainCreds),
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

enum Status {
    Allowed,
    Denied,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Status::Allowed => write!(f, "200"),
            Status::Denied => write!(f, "400"),
        }
    }
}

struct ZapRequest {
    version: String,
    request_id: Msg,
    domain: String,
    addr: IpAddr,
    routing_id: RoutingId,
    mechanism: String,
    credentials: Vec<Msg>,
}

impl ZapRequest {
    fn new(mut parts: Vec<Msg>) -> Self {
        let version = parts.pop().unwrap().to_str().unwrap().to_owned();
        assert_eq!(version, ZAP_VERSION);

        let request_id = parts.pop().unwrap().to_owned();
        let domain = parts.pop().unwrap().to_str().unwrap().to_owned();
        let addr: IpAddr =
            parts.pop().unwrap().to_str().unwrap().parse().unwrap();

        let routing_id =
            RoutingId::from_str(parts.pop().unwrap().to_str().unwrap())
                .unwrap();

        let mechanism = parts.pop().unwrap().to_str().unwrap().to_owned();

        Self {
            version,
            request_id,
            domain,
            addr,
            routing_id,
            mechanism,
            credentials: parts,
        }
    }
}

struct ZapReply {
    request_id: Msg, //  Sequence number of request
    user_id: String,
    metadata: Vec<u8>,
    version: String, //  Version number, must be "1.0"
    status_code: Status,
    status_text: String,
}

struct Authenticator {
    //  ZAP handler socket
    handler: Dealer,
    command: Server,
    verbose: bool,
    whitelist: HashSet<IpAddr>,
    blacklist: HashSet<IpAddr>,
    passwords: HashMap<String, String>,
}

struct AuthResult {
    user_id: String,
    metadata: Vec<u8>,
}

impl Authenticator {
    pub fn new() -> Result<Self, Error> {
        Self::with_ctx(Ctx::global())
    }

    pub fn with_ctx<C>(ctx: C) -> Result<Self, Error>
    where
        C: Into<Ctx>,
    {
        let ctx = ctx.into();
        let handler = Dealer::with_ctx(&ctx)?;
        let command = Server::with_ctx(ctx)?;

        Ok(Self {
            handler,
            command,
            verbose: false,
            whitelist: HashSet::default(),
            blacklist: HashSet::default(),
            passwords: HashMap::default(),
        })
    }

    fn run(&mut self) -> Result<(), Error> {
        let mut poller = Poller::new();
        poller.add(&self.handler, PollId(0), READABLE)?;
        poller.add(&self.command, PollId(1), READABLE)?;

        let mut events = Events::new();

        loop {
            poller.block(&mut events, None)?;

            for event in &events {
                if event.flags() == READABLE {
                    match event.id() {
                        PollId(0) => {
                            let _reply = self.on_zap()?;
                            unimplemented!()
                        }
                        PollId(1) => unimplemented!(),
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    fn on_zap(&mut self) -> Result<ZapReply, Error> {
        let parts = self.handler.recv_msg_multipart()?;
        let mut request = ZapRequest::new(parts);

        assert_eq!(request.version, ZAP_VERSION);

        let mut denied = false;

        if !self.whitelist.is_empty() {
            if !self.whitelist.contains(&request.addr) {
                denied = true;
            }
        }

        if !self.blacklist.is_empty() && !denied {
            if self.blacklist.contains(&request.addr) {
                denied = true;
            }
        }

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
                                .pop()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_owned();

                            let password = request
                                .credentials
                                .pop()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_owned();

                            let creds = PlainCreds { username, password };
                            self.auth_plain(creds)
                        }
                        _ => unimplemented!(),
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
                status_code: Status::Allowed,
                status_text: "Allowed".to_owned(),
            })
        } else {
            Ok(ZapReply {
                request_id: request.request_id,
                user_id: String::new(),
                metadata: vec![],
                version: ZAP_VERSION.to_owned(),
                status_code: Status::Denied,
                status_text: "Denied".to_owned(),
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
    use crate::{auth::*, prelude::*, socket::*};

    use hashbrown::HashMap;

    use std::convert::TryInto;

    #[test]
    fn test_null_mechanism() {
        let ctx = Ctx::new();
        let addr: InprocAddr = "test".try_into().unwrap();

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::Null)
            .with_ctx(&ctx)
            .unwrap();

        let client = ClientBuilder::new()
            .connect(addr)
            .mechanism(Mechanism::Null)
            .with_ctx(&ctx)
            .unwrap();
    }

    #[test]
    fn test_null_plain() {
        let ctx = Ctx::new();
        let addr: InprocAddr = "test".try_into().unwrap();

        let mut map = HashMap::new();
        map.insert("jane", "doe");

        let server = ServerBuilder::new()
            .bind(&addr)
            .mechanism(Mechanism::PlainServer)
            .with_ctx(&ctx)
            .unwrap();

        let creds = PlainCreds {
            username: "ok".to_owned(),
            password: "lo".to_owned(),
        };
        let client = ClientBuilder::new()
            .connect(addr)
            .mechanism(Mechanism::PlainClient(creds))
            .with_ctx(&ctx)
            .unwrap();
    }
}
