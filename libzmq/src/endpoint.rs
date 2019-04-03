use serde::{Deserialize, Serialize};

use failure::Fail;

use std::{borrow::Cow, fmt, str};

/// For the moment this is simply a wrapper around a `String`.
///
/// [See this issue](https://github.com/jean-airoldie/libzmq-rs/issues/6)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TcpAddr {
    addr: String,
}

impl str::FromStr for TcpAddr {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { addr: s.to_owned() })
    }
}

impl fmt::Display for TcpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}

/// For the moment this is simply a wrapper around a `String`.
///
/// [See this issue](https://github.com/jean-airoldie/libzmq-rs/issues/7)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct IpcAddr {
    addr: String,
}

impl str::FromStr for IpcAddr {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { addr: s.to_owned() })
    }
}

impl fmt::Display for IpcAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}

/// For the moment this is simply a wrapper around a `String`.
///
/// [See this issue](https://github.com/jean-airoldie/libzmq-rs/issues/8)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct InprocAddr {
    addr: String,
}

impl str::FromStr for InprocAddr {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { addr: s.to_owned() })
    }
}

impl fmt::Display for InprocAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}

/// For the moment this is simply a wrapper around a `String`.
///
/// [See this issue](https://github.com/jean-airoldie/libzmq-rs/issues/9)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PgmAddr {
    addr: String,
}

impl str::FromStr for PgmAddr {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { addr: s.to_owned() })
    }
}

impl fmt::Display for PgmAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}
/// For the moment this is simply a wrapper around a `String`.
///
/// [See this issue](https://github.com/jean-airoldie/libzmq-rs/issues/10)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct EpgmAddr {
    addr: String,
}

impl str::FromStr for EpgmAddr {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { addr: s.to_owned() })
    }
}

impl fmt::Display for EpgmAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}

/// For the moment this is simply a wrapper around a `String`.
///
/// [See this issue](https://github.com/jean-airoldie/libzmq-rs/issues/11)
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct VmciAddr {
    addr: String,
}

impl str::FromStr for VmciAddr {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self { addr: s.to_owned() })
    }
}

impl fmt::Display for VmciAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.addr)
    }
}

/// A transport and a transport-specific address supported by ØMQ.
///
/// Currently this enum does not provide any functionality and is
/// nothing more than an interface around `String`. Proper parsing
/// will eventualy be added, (hopefully) without modifying the interface.
/// [See this issue] (https://github.com/jean-airoldie/libzmq-rs/issues/5)
/// for more information.
///
/// # Bind vs. Connect
/// For most transports and socket types the connection is not performed
/// immediately but as needed by ØMQ. Thus a successful call to `connect`
/// does not mean that the connection was or could actually be established.
/// Because of this, for most transports and socket types the order in
/// which a server socket is bound and a client socket is connected to it
/// does not matter.
///
/// # Endpoint
/// The endpoint is a string consisting of a `"{transport}://{address}"`,
/// e.g. `"tcp://localhost:3000"`.
/// The transport specifies the underlying protocol to use.
/// The address specifies the transport-specific address to connect to.
///
/// # Summary of Transports
/// | Transport `str` | Description                                 | Reference      |
/// | ----------------|:-------------------------------------------:|:--------------:|
/// | "tcp"           | unicast transport using TCP                 | [`zmq_tcp`]    |
/// | "ipc"           | local inter-process communication transport | [`zmq_ipc`]    |
/// | "inproc"        | local in-process communication transport    | [`zmq_inproc`] |
/// | "pgm", "epgm"   | reliable multicast transport using PGM      | [`zmq_pgm`]    |
/// | "vmci"          | virtual machine communications interface    | [`zmq_vmci`]   |
///
/// ```
/// use libzmq::{prelude::*};
/// use Endpoint::*;
///
/// // IPv4 addr with TCP transport.
/// let endpoint: Endpoint = "tcp://127.0.0.1:9090".parse().unwrap();
///
/// assert!(endpoint.is_tcp());
/// ```
///
/// This enum type is non-exhaustive and could have additional variants
/// added in future. Therefore, when matching against variants of
/// non-exhaustive enums, an extra wildcard arm must be added to account
/// for any future variants.
///
/// [`zmq_tcp`]: http://api.zeromq.org/master:zmq_tcp
/// [`zmq_ipc`]: http://api.zeromq.org/master:zmq_ipc
/// [`zmq_inproc`]: http://api.zeromq.org/master:zmq_inproc
/// [`zmq_pgm`]: http://api.zeromq.org/master:zmq_pgm
/// [`zmq_vmci`]: http://api.zeromq.org/master:zmq_vmci
/// # Usage Example
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Endpoint {
    /// Unicast transport using TCP, see [`zmq_tcp`].
    ///
    /// [`zmq_tcp`]: http://api.zeromq.org/master:zmq-tcp
    Tcp(TcpAddr),
    /// Local inter-process communication transport, see [`zmq_ipc`].
    ///
    /// [`zmq_ipc`]: http://api.zeromq.org/master:zmq-ipc
    Ipc(IpcAddr),
    /// Local in-process (inter-thread) communication transport,
    /// see [`zmq_inproc`].
    ///
    /// [`zmq_inproc`]: http://api.zeromq.org/master:zmq-inproc
    Inproc(InprocAddr),
    /// Reliable multicast transport using PGM, see [`zmq_pgm`].
    ///
    /// [`zmq_pgm`]: http://api.zeromq.org/master:zmq-pgm
    Pgm(PgmAddr),
    /// Reliable multicast transport using EPGM, see [`zmq_pgm`].
    ///
    /// [`zmq_pgm`]: http://api.zeromq.org/master:zmq-pgm
    Epgm(EpgmAddr),
    /// Virtual machine communications interface (VMCI), see [`zmq_vmci`].
    ///
    /// [`zmq_vmci`]: http://api.zeromq.org/master:zmq-vmci
    Vmci(VmciAddr),
}

impl Endpoint {
    /// Returns `true` if the endpoint uses the `Tcp` transport.
    pub fn is_tcp(&self) -> bool {
        if let Endpoint::Tcp(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the endpoint uses the `Ipc` transport.
    pub fn is_ipc(&self) -> bool {
        if let Endpoint::Ipc(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the endpoint uses the `Inproc` transport.
    pub fn is_inproc(&self) -> bool {
        if let Endpoint::Inproc(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the endpoint uses the `Pgm` transport.
    pub fn is_pgm(&self) -> bool {
        if let Endpoint::Pgm(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the endpoint uses the `Epgm` transport.
    pub fn is_edpgm(&self) -> bool {
        if let Endpoint::Epgm(_) = self {
            true
        } else {
            false
        }
    }

    /// Returns `true` if the endpoint uses the `Vmci` transport.
    pub fn is_vmci(&self) -> bool {
        if let Endpoint::Vmci(_) = self {
            true
        } else {
            false
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::Tcp(addr) => write!(f, "tcp://{}", addr),
            Endpoint::Ipc(addr) => write!(f, "ipc://{}", addr),
            Endpoint::Inproc(addr) => write!(f, "inproc://{}", addr),
            Endpoint::Pgm(addr) => write!(f, "pgm://{}", addr),
            Endpoint::Epgm(addr) => write!(f, "epgm://{}", addr),
            Endpoint::Vmci(addr) => write!(f, "vmci://{}", addr),
        }
    }
}

/// An error that occurs when an `Endpoint` could not be parsed.
///
/// [`Endpoint`]: enum.Endpoint.html
#[derive(Debug, Fail)]
#[fail(display = "unable to parse endpoint: {}", msg)]
pub struct EndpointParseError {
    msg: String,
}

impl str::FromStr for Endpoint {
    type Err = EndpointParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = s.split("://").collect();
        if parts.len() == 2 {
            let addr = parts.pop().unwrap();
            let transport = parts.pop().unwrap();

            match transport {
                "tcp" => Ok(Endpoint::Tcp(addr.parse()?)),
                "ipc" => Ok(Endpoint::Ipc(addr.parse()?)),
                "inproc" => Ok(Endpoint::Inproc(addr.parse()?)),
                "pgm" => Ok(Endpoint::Pgm(addr.parse()?)),
                "epgm" => Ok(Endpoint::Epgm(addr.parse()?)),
                "vmci" => Ok(Endpoint::Vmci(addr.parse()?)),
                _ => Err(EndpointParseError {
                    msg: format!("invalid transport : {}", transport),
                }),
            }
        } else {
            Err(EndpointParseError {
                msg: format!("malformed endpoint"),
            })
        }
    }
}

impl AsRef<Endpoint> for Endpoint {
    fn as_ref(&self) -> &Endpoint {
        self
    }
}
