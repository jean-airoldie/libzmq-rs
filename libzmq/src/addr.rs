//! The different transport types supported by ØMQ.

use failure::Fail;
use serde::{Deserialize, Serialize};

use std::{
    convert::{Infallible, TryFrom},
    fmt,
    net::{AddrParseError, IpAddr, SocketAddr},
    option,
    str::{self, FromStr},
};

pub const INPROC_MAX_SIZE: usize = 256;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TcpAddr {
    inner: SocketAddr,
}

impl TcpAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let inner = SocketAddr::new(ip, port);

        Self { inner }
    }

    pub fn ip(&self) -> IpAddr {
        self.inner.ip()
    }

    pub fn set_ip(&mut self, new_ip: IpAddr) {
        self.inner.set_ip(new_ip);
    }

    pub fn port(&self) -> u16 {
        self.inner.port()
    }

    pub fn set_port(&mut self, new_port: u16) {
        self.inner.set_port(new_port)
    }

    pub fn is_ipv4(&self) -> bool {
        self.inner.is_ipv4()
    }

    pub fn is_ipv6(&self) -> bool {
        self.inner.is_ipv6()
    }
}

impl From<SocketAddr> for TcpAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { inner: addr }
    }
}

impl From<TcpAddr> for SocketAddr {
    fn from(addr: TcpAddr) -> Self {
        addr.inner
    }
}

impl fmt::Display for TcpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tcp://{:?}", self.inner)
    }
}

impl FromStr for TcpAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<TcpAddr, AddrParseError> {
        let inner = SocketAddr::from_str(s)?;
        Ok(Self { inner })
    }
}

impl TryFrom<String> for TcpAddr {
    type Error = AddrParseError;
    fn try_from(s: String) -> Result<TcpAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a String> for TcpAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a String) -> Result<TcpAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for TcpAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a str) -> Result<TcpAddr, AddrParseError> {
        Self::from_str(s)
    }
}

impl IntoIterator for TcpAddr {
    type Item = TcpAddr;
    type IntoIter = option::IntoIter<TcpAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a TcpAddr {
    type Item = &'a TcpAddr;
    type IntoIter = option::IntoIter<&'a TcpAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl From<TcpAddr> for Endpoint {
    fn from(addr: TcpAddr) -> Endpoint {
        Endpoint::Tcp(addr)
    }
}

impl<'a> From<&'a TcpAddr> for Endpoint {
    fn from(addr: &'a TcpAddr) -> Endpoint {
        Endpoint::Tcp(addr.to_owned())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UdpAddr {
    inner: SocketAddr,
}

impl UdpAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let inner = SocketAddr::new(ip, port);

        Self { inner }
    }

    pub fn ip(&self) -> IpAddr {
        self.inner.ip()
    }

    pub fn set_ip(&mut self, new_ip: IpAddr) {
        self.inner.set_ip(new_ip);
    }

    pub fn port(&self) -> u16 {
        self.inner.port()
    }

    pub fn set_port(&mut self, new_port: u16) {
        self.inner.set_port(new_port)
    }

    pub fn is_ipv4(&self) -> bool {
        self.inner.is_ipv4()
    }

    pub fn is_ipv6(&self) -> bool {
        self.inner.is_ipv6()
    }
}

impl From<SocketAddr> for UdpAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { inner: addr }
    }
}

impl From<UdpAddr> for SocketAddr {
    fn from(addr: UdpAddr) -> Self {
        addr.inner
    }
}

impl fmt::Display for UdpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "udp://{:?}", self.inner)
    }
}

impl FromStr for UdpAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<UdpAddr, AddrParseError> {
        let inner = SocketAddr::from_str(s)?;
        Ok(Self { inner })
    }
}

impl TryFrom<String> for UdpAddr {
    type Error = AddrParseError;
    fn try_from(s: String) -> Result<UdpAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a String> for UdpAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a String) -> Result<UdpAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for UdpAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a str) -> Result<UdpAddr, AddrParseError> {
        Self::from_str(s)
    }
}

impl IntoIterator for UdpAddr {
    type Item = UdpAddr;
    type IntoIter = option::IntoIter<UdpAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a UdpAddr {
    type Item = &'a UdpAddr;
    type IntoIter = option::IntoIter<&'a UdpAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl From<UdpAddr> for Endpoint {
    fn from(addr: UdpAddr) -> Endpoint {
        Endpoint::Udp(addr)
    }
}

impl<'a> From<&'a UdpAddr> for Endpoint {
    fn from(addr: &'a UdpAddr) -> Endpoint {
        Endpoint::Udp(addr.to_owned())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PgmAddr {
    inner: SocketAddr,
}

impl PgmAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let inner = SocketAddr::new(ip, port);

        Self { inner }
    }

    pub fn ip(&self) -> IpAddr {
        self.inner.ip()
    }

    pub fn set_ip(&mut self, new_ip: IpAddr) {
        self.inner.set_ip(new_ip);
    }

    pub fn port(&self) -> u16 {
        self.inner.port()
    }

    pub fn set_port(&mut self, new_port: u16) {
        self.inner.set_port(new_port)
    }

    pub fn is_ipv4(&self) -> bool {
        self.inner.is_ipv4()
    }

    pub fn is_ipv6(&self) -> bool {
        self.inner.is_ipv6()
    }
}

impl From<SocketAddr> for PgmAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { inner: addr }
    }
}

impl From<PgmAddr> for SocketAddr {
    fn from(addr: PgmAddr) -> Self {
        addr.inner
    }
}

impl fmt::Display for PgmAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pgm://{:?}", self.inner)
    }
}

impl FromStr for PgmAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<PgmAddr, AddrParseError> {
        let inner = SocketAddr::from_str(s)?;
        Ok(Self { inner })
    }
}

impl TryFrom<String> for PgmAddr {
    type Error = AddrParseError;
    fn try_from(s: String) -> Result<PgmAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a String> for PgmAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a String) -> Result<PgmAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for PgmAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a str) -> Result<PgmAddr, AddrParseError> {
        Self::from_str(s)
    }
}

impl IntoIterator for PgmAddr {
    type Item = PgmAddr;
    type IntoIter = option::IntoIter<PgmAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a PgmAddr {
    type Item = &'a PgmAddr;
    type IntoIter = option::IntoIter<&'a PgmAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl From<PgmAddr> for Endpoint {
    fn from(addr: PgmAddr) -> Endpoint {
        Endpoint::Pgm(addr)
    }
}

impl<'a> From<&'a PgmAddr> for Endpoint {
    fn from(addr: &'a PgmAddr) -> Endpoint {
        Endpoint::Pgm(addr.to_owned())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EpgmAddr {
    inner: SocketAddr,
}

impl EpgmAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        let inner = SocketAddr::new(ip, port);

        Self { inner }
    }

    pub fn ip(&self) -> IpAddr {
        self.inner.ip()
    }

    pub fn set_ip(&mut self, new_ip: IpAddr) {
        self.inner.set_ip(new_ip);
    }

    pub fn port(&self) -> u16 {
        self.inner.port()
    }

    pub fn set_port(&mut self, new_port: u16) {
        self.inner.set_port(new_port)
    }

    pub fn is_ipv4(&self) -> bool {
        self.inner.is_ipv4()
    }

    pub fn is_ipv6(&self) -> bool {
        self.inner.is_ipv6()
    }
}

impl From<SocketAddr> for EpgmAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { inner: addr }
    }
}

impl From<EpgmAddr> for SocketAddr {
    fn from(addr: EpgmAddr) -> Self {
        addr.inner
    }
}

impl fmt::Display for EpgmAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "epgm://{:?}", self.inner)
    }
}

impl FromStr for EpgmAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<EpgmAddr, AddrParseError> {
        let inner = SocketAddr::from_str(s)?;
        Ok(Self { inner })
    }
}

impl TryFrom<String> for EpgmAddr {
    type Error = AddrParseError;
    fn try_from(s: String) -> Result<EpgmAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a String> for EpgmAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a String) -> Result<EpgmAddr, AddrParseError> {
        Self::from_str(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for EpgmAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a str) -> Result<EpgmAddr, AddrParseError> {
        Self::from_str(s)
    }
}

impl IntoIterator for EpgmAddr {
    type Item = EpgmAddr;
    type IntoIter = option::IntoIter<EpgmAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a EpgmAddr {
    type Item = &'a EpgmAddr;
    type IntoIter = option::IntoIter<&'a EpgmAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl From<EpgmAddr> for Endpoint {
    fn from(addr: EpgmAddr) -> Endpoint {
        Endpoint::Epgm(addr)
    }
}

impl<'a> From<&'a EpgmAddr> for Endpoint {
    fn from(addr: &'a EpgmAddr) -> Endpoint {
        Endpoint::Epgm(addr.to_owned())
    }
}

/// A wrapper for `String` that is passed directly to `libzmq` without
/// type checking.
///
/// This is meant to allow usage of features not currently supported by
/// endpoint types such as:
/// * The wild-card *, meaning all available interfaces.
/// * Non-portable interface name as defined by the operating system.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawAddr {
    addr: String,
}

impl RawAddr {
    pub fn new<S>(addr: S) -> Self
    where
        S: Into<String>,
    {
        let addr = addr.into();
        Self { addr }
    }

    pub fn as_str(&self) -> &str {
        self.addr.as_str()
    }
}

impl fmt::Display for RawAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RawAddr {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<RawAddr, Infallible> {
        Ok(Self::new(s))
    }
}

impl IntoIterator for RawAddr {
    type Item = RawAddr;
    type IntoIter = option::IntoIter<RawAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a RawAddr {
    type Item = &'a RawAddr;
    type IntoIter = option::IntoIter<&'a RawAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl From<RawAddr> for Endpoint {
    fn from(addr: RawAddr) -> Endpoint {
        Endpoint::Raw(addr)
    }
}

impl<'a> From<&'a RawAddr> for Endpoint {
    fn from(addr: &'a RawAddr) -> Endpoint {
        Endpoint::Raw(addr.to_owned())
    }
}

#[derive(Debug, Fail)]
#[fail(display = "inproc addr cannot exceed INPROC_MAX_SIZE char")]
pub struct InprocAddrError;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InprocAddr {
    addr: String,
}

impl InprocAddr {
    pub fn new<S>(addr: S) -> Result<Self, InprocAddrError>
    where
        S: Into<String>,
    {
        let addr = addr.into();
        if addr.len() <= INPROC_MAX_SIZE {
            Ok(Self { addr })
        } else {
            Err(InprocAddrError)
        }
    }

    pub fn as_str(&self) -> &str {
        self.addr.as_str()
    }
}

impl PartialEq<str> for InprocAddr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<InprocAddr> for str {
    fn eq(&self, other: &InprocAddr) -> bool {
        other.as_str() == self
    }
}

impl<'a> PartialEq<&'a str> for InprocAddr {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_str() == *other
    }
}

impl<'a> PartialEq<InprocAddr> for &'a str {
    fn eq(&self, other: &InprocAddr) -> bool {
        other.as_str() == *self
    }
}

impl TryFrom<String> for InprocAddr {
    type Error = InprocAddrError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl<'a> TryFrom<&'a String> for InprocAddr {
    type Error = InprocAddrError;
    fn try_from(s: &'a String) -> Result<Self, Self::Error> {
        Self::new(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for InprocAddr {
    type Error = InprocAddrError;
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl fmt::Display for InprocAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "inproc://{}", self.as_str())
    }
}

impl FromStr for InprocAddr {
    type Err = InprocAddrError;
    fn from_str(s: &str) -> Result<InprocAddr, InprocAddrError> {
        Self::new(s)
    }
}

impl IntoIterator for InprocAddr {
    type Item = InprocAddr;
    type IntoIter = option::IntoIter<InprocAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a InprocAddr {
    type Item = &'a InprocAddr;
    type IntoIter = option::IntoIter<&'a InprocAddr>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl From<InprocAddr> for Endpoint {
    fn from(addr: InprocAddr) -> Endpoint {
        Endpoint::Inproc(addr)
    }
}

impl<'a> From<&'a InprocAddr> for Endpoint {
    fn from(addr: &'a InprocAddr) -> Endpoint {
        Endpoint::Inproc(addr.to_owned())
    }
}

/// A transport and a transport-specific address supported by ØMQ.
///
/// The transport specifies the underlying protocol to use. The address
/// specifies the transport-specific address to connect to.
///
/// # Bind vs. Connect
/// For most transports and socket types the connection is not performed
/// immediately but as needed by ØMQ. Thus a successful call to `connect`
/// does not mean that the connection was or could actually be established.
/// Because of this, for most transports and socket types the order in
/// which a server socket is bound and a client socket is connected to it
/// does not matter.
///
/// # Summary of Transports
/// | Transport `str` | Description                                 | Reference      |
/// | ----------------|:-------------------------------------------:|:--------------:|
/// | "tcp"           | unicast transport using TCP                 | [`zmq_tcp`]    |
/// | "udp"           | UDP multicast and unicast transport         | [`zmq_udp`]    |
/// | "ipc"           | local inter-process communication transport | [`zmq_ipc`]    |
/// | "inproc"        | local in-process communication transport    | [`zmq_inproc`] |
/// | "pgm", "epgm"   | reliable multicast transport using PGM      | [`zmq_pgm`]    |
/// | "vmci"          | virtual machine communications interface    | [`zmq_vmci`]   |
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::{TcpAddr, Endpoint};
/// use std::convert::TryInto;
///
/// // IPv4 addr with TCP transport.
/// let addr: TcpAddr = "127.0.0.1:9090".try_into()?;
/// assert!(addr.is_ipv4());
///
/// let endpoint: Endpoint = addr.into();
/// assert!(endpoint.is_tcp());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// This enum type is non-exhaustive and could have additional variants
/// added in future. Therefore, when matching against variants of
/// non-exhaustive enums, an extra wildcard arm must be added to account
/// for any future variants.
///
/// [`zmq_tcp`]: http://api.zeromq.org/master:zmq_tcp
/// [`zmq_udp`]: http://api.zeromq.org/master:zmq-udp
/// [`zmq_ipc`]: http://api.zeromq.org/master:zmq_ipc
/// [`zmq_inproc`]: http://api.zeromq.org/master:zmq_inproc
/// [`zmq_pgm`]: http://api.zeromq.org/master:zmq_pgm
/// [`zmq_vmci`]: http://api.zeromq.org/master:zmq_vmci
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Endpoint {
    /// Unicast transport using TCP, see [`zmq_tcp`].
    ///
    /// [`zmq_tcp`]: http://api.zeromq.org/master:zmq-tcp
    Tcp(TcpAddr),
    /// ØMQ UDP multicast and unicast transport
    ///
    /// [`zmq_udp`]: http://api.zeromq.org/master:zmq-udp
    Udp(UdpAddr),
    /// Local in-process (inter-thread) communication transport
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
    /// A raw `String` directly passed to libzmq used to bypass
    /// type checking.
    Raw(RawAddr),
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

    /// Returns `true` if the endpoint uses the `Ucp` transport.
    pub fn is_udp(&self) -> bool {
        if let Endpoint::Udp(_) = self {
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

    /// Returns `true` if the endpoint is a `RawAddr`.
    pub fn is_raw(&self) -> bool {
        if let Endpoint::Raw(_) = self {
            true
        } else {
            false
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::Tcp(addr) => write!(f, "{}", addr),
            Endpoint::Udp(addr) => write!(f, "{}", addr),
            Endpoint::Inproc(addr) => write!(f, "{}", addr),
            Endpoint::Pgm(addr) => write!(f, "{}", addr),
            Endpoint::Epgm(addr) => write!(f, "{}", addr),
            Endpoint::Raw(addr) => write!(f, "{}", addr),
        }
    }
}

impl IntoIterator for Endpoint {
    type Item = Endpoint;
    type IntoIter = option::IntoIter<Endpoint>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a Endpoint {
    type Item = &'a Endpoint;
    type IntoIter = option::IntoIter<&'a Endpoint>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}
