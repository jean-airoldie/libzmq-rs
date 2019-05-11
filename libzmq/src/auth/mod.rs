mod dealer;

pub(crate) use dealer::Dealer;

use crate::{poll::*, Error, Msg, RoutingId, Server};

use failure::Fail;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use std::{convert::TryFrom, fmt, net::IpAddr};

const ZAP_VERSION: &str = "1.0";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mechanism {
    Null = 0,
    Plain,
}

impl From<u8> for Mechanism {
    fn from(u: u8) -> Self {
        match u {
            u if u == Mechanism::Null as u8 => Mechanism::Null,
            u if u == Mechanism::Plain as u8 => Mechanism::Plain,
            _ => unreachable!(),
        }
    }
}

impl From<Mechanism> for u8 {
    fn from(r: Mechanism) -> Self {
        match r {
            Mechanism::Null => Mechanism::Null as u8,
            Mechanism::Plain => Mechanism::Plain as u8,
        }
    }
}

impl Default for Mechanism {
    fn default() -> Self {
        Mechanism::Null
    }
}

#[derive(Debug, Fail)]
#[fail(display = "unsupported mechanism")]
pub struct InvalidMechanism;

impl<'a> TryFrom<&'a str> for Mechanism {
    type Error = InvalidMechanism;

    fn try_from(s: &'a str) -> Result<Mechanism, InvalidMechanism> {
        match s {
            "NULL" => Ok(Mechanism::Null),
            "PLAIN" => Ok(Mechanism::Plain),
            _ => Err(InvalidMechanism),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Creds {
    Null,
    Plain(PlainCreds),
}

impl From<PlainCreds> for Creds {
    fn from(creds: PlainCreds) -> Self {
        Creds::Plain(creds)
    }
}

impl<'a> From<&'a PlainCreds> for Creds {
    fn from(creds: &'a PlainCreds) -> Self {
        Creds::Plain(creds.to_owned())
    }
}

impl<'a> From<&'a Creds> for Creds {
    fn from(creds: &'a Creds) -> Self {
        creds.to_owned()
    }
}

impl Default for Creds {
    fn default() -> Self {
        Creds::Null
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlainCreds {
    pub username: String,
    pub password: String,
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
    pub fn new(handler: Dealer, command: Server) -> Self {
        Authenticator {
            handler,
            command,
            verbose: false,
            whitelist: HashSet::default(),
            blacklist: HashSet::default(),
            passwords: HashMap::default(),
        }
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
                Mechanism::try_from(request.mechanism.as_str())
            {
                result = {
                    match mechanism {
                        Mechanism::Null => Some(AuthResult {
                            user_id: String::new(),
                            metadata: vec![],
                        }),
                        Mechanism::Plain => {
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
    use crate::{auth::Mechanism, prelude::*, socket::*, *};

    use hashbrown::HashMap;

    use std::convert::TryInto;

    #[test]
    fn test_null_mechanism() {
        let ctx = Ctx::new();
        let addr: InprocAddr = "test".try_into().unwrap();

        //let zap = Authenticator::new();

        let _server = ServerBuilder::new()
            .bind(&addr)
            .auth_role(AuthRole::Server)
            .mechanism(Mechanism::Null)
            .build_with_ctx(&ctx)
            .unwrap();

        let _client = ClientBuilder::new()
            .connect(addr)
            .auth_role(AuthRole::Client)
            .mechanism(Mechanism::Null)
            .build_with_ctx(&ctx)
            .unwrap();
    }

    #[test]
    fn test_null_plain() {
        let ctx = Ctx::new();
        let addr: InprocAddr = "test".try_into().unwrap();

        let mut map = HashMap::new();
        map.insert("jane", "doe");

        //let zap = AuthHandlerBuilder::new()
        //    .plain(map)
        //    .build()?;

        let _server = ServerBuilder::new()
            .bind(&addr)
            .auth_role(AuthRole::Server)
            .mechanism(Mechanism::Plain)
            .build_with_ctx(&ctx)
            .unwrap();

        let _client = ClientBuilder::new()
            .connect(addr)
            .auth_role(AuthRole::Client)
            .mechanism(Mechanism::Plain)
            .build_with_ctx(&ctx)
            .unwrap();
    }
}
