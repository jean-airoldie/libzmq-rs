//! The different transport types supported by ØMQ.

use failure::Fail;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use std::{
    convert::TryFrom,
    fmt,
    net::{self, IpAddr},
    option,
    str::{self, FromStr},
};

/// The maximum number of characters in a `inproc` address.
pub const INPROC_MAX_SIZE: usize = 256;

/// An error that occurs when an address cannot be parsed.
///
/// The error contains a message detailling the source of the error.
#[derive(Debug, Fail)]
#[fail(display = "cannot parse address : {}", msg)]
pub struct AddrParseError {
    msg: &'static str,
}

impl AddrParseError {
    fn new(msg: &'static str) -> Self {
        Self { msg }
    }

    pub fn msg(&self) -> &'static str {
        self.msg
    }
}

macro_rules! serde_display_tryfrom {
    ($name:ident) => {
        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                TryFrom::try_from(s).map_err(de::Error::custom)
            }
        }
    };
}

macro_rules! tryfrom_fromstr {
    ($name:ident) => {
        impl TryFrom<String> for $name {
            type Error = AddrParseError;
            fn try_from(s: String) -> Result<Self, AddrParseError> {
                Self::from_str(s.as_str())
            }
        }

        impl<'a> TryFrom<&'a String> for $name {
            type Error = AddrParseError;
            fn try_from(s: &'a String) -> Result<Self, AddrParseError> {
                Self::from_str(s.as_str())
            }
        }

        impl<'a> TryFrom<&'a str> for $name {
            type Error = AddrParseError;
            fn try_from(s: &'a str) -> Result<Self, AddrParseError> {
                Self::from_str(s)
            }
        }
    };
}

/// An named interface.
///
/// It can represente a network interface, a DNS address or
/// a IP hostname depending on the context.
///
/// The hostname must be strictly alpha-numeric except for the `-` character
/// that is allowed.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::Hostname;
/// use std::convert::TryInto;
///
/// // This is a network interface.
/// let net: Hostname = "eth0".try_into()?;
/// // This is a DNS address.
/// let dns: Hostname = "server-name".try_into()?;
/// // This is a IPv4 hostname
/// let localhost: Hostname = "localhost".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Hostname {
    name: String,
}

impl Hostname {
    pub fn new<S>(name: S) -> Result<Self, AddrParseError>
    where
        S: Into<String>,
    {
        let name = name.into();

        if !name.is_empty() {
            for c in name.as_str().chars() {
                if !c.is_ascii_alphanumeric() && c != '-' {
                    dbg!(c);
                    return Err(AddrParseError::new(
                        "hostname contains illegal char",
                    ));
                }
            }

            Ok(Self { name })
        } else {
            Err(AddrParseError::new("empty hostname"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }
}

impl FromStr for Hostname {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        Self::new(s)
    }
}

impl fmt::Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

serde_display_tryfrom!(Hostname);

impl TryFrom<String> for Hostname {
    type Error = AddrParseError;
    fn try_from(s: String) -> Result<Self, AddrParseError> {
        Self::new(s)
    }
}

impl<'a> TryFrom<&'a String> for Hostname {
    type Error = AddrParseError;
    fn try_from(s: &'a String) -> Result<Self, AddrParseError> {
        Self::new(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for Hostname {
    type Error = AddrParseError;
    fn try_from(s: &'a str) -> Result<Self, AddrParseError> {
        Self::new(s)
    }
}

/// A port used by a socket address.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::Port;
/// use std::convert::TryInto;
///
/// let port: Port = "*".try_into()?;
/// assert_eq!(port, Port::Dynamic);
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Port {
    /// A specific port number.
    Static(u16),
    /// A system assigned ephemeral port.
    Dynamic,
}

impl FromStr for Port {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if !s.is_empty() {
            if s.chars().nth(0).unwrap() == '*' && s.len() == 1 {
                Ok(Port::Dynamic)
            } else {
                let port = u16::from_str(s)
                    .map_err(|_| AddrParseError::new("invalid port number"))?;
                Ok(Port::Static(port))
            }
        } else {
            Err(AddrParseError::new("empty port"))
        }
    }
}

tryfrom_fromstr!(Port);

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Port::Static(num) => write!(f, "{}", num),
            Port::Dynamic => write!(f, "*"),
        }
    }
}

serde_display_tryfrom!(Port);

/// An interface used for connecting or binding.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::Interface;
/// use std::convert::TryInto;
///
/// let interface: Interface = "0.0.0.0".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Interface {
    /// Connect or bind to an IP address.
    Ip(IpAddr),
    /// Connect or bind to a named address.
    Hostname(Hostname),
}

impl FromStr for Interface {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if !s.is_empty() {
            dbg!(s);
            if let Ok(ip) = IpAddr::from_str(s) {
                Ok(Interface::Ip(ip))
            } else {
                let interface = Hostname::from_str(s)?;
                Ok(Interface::Hostname(interface))
            }
        } else {
            Err(AddrParseError::new("empty interface"))
        }
    }
}

tryfrom_fromstr!(Interface);

impl fmt::Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Interface::Ip(ip) => write!(f, "{}", ip),
            Interface::Hostname(interface) => write!(f, "{}", interface),
        }
    }
}

serde_display_tryfrom!(Interface);

/// A socket address with an [`Interface`] and a [`Port`].
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::SocketAddr;
/// use std::convert::TryInto;
///
/// let addr: SocketAddr = "127.0.0.1:3000".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`Interface`]: enum.Interface.html
/// [`Port`]: enum.Port.html
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SocketAddr {
    interface: Interface,
    port: Port,
}

impl SocketAddr {
    pub fn new(interface: Interface, port: Port) -> Self {
        Self { interface, port }
    }

    pub fn interface(&self) -> &Interface {
        &self.interface
    }

    pub fn port(&self) -> Port {
        self.port
    }
}

impl FromStr for SocketAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        // We reverse search so that we don't have to deal will IPv6 syntax.
        if let Some(mid) = s.rfind(':') {
            let addr = {
                // Check for IPv6.
                if s.chars().nth(0).unwrap() == '['
                    && s.chars().nth(mid - 1).unwrap() == ']'
                {
                    let interface = Interface::from_str(&s[1..mid - 1])?;
                    let port = Port::from_str(&s[mid + 1..])?;

                    Self { interface, port }
                } else {
                    let interface = Interface::from_str(&s[0..mid])?;
                    let port = Port::from_str(&s[mid + 1..])?;

                    Self { interface, port }
                }
            };

            Ok(addr)
        } else {
            Err(AddrParseError::new("invalid addr format"))
        }
    }
}

tryfrom_fromstr!(SocketAddr);

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.interface, self.port)
    }
}

serde_display_tryfrom!(SocketAddr);

impl From<net::SocketAddr> for SocketAddr {
    fn from(addr: net::SocketAddr) -> Self {
        Self::new(Interface::Ip(addr.ip()), Port::Static(addr.port()))
    }
}

impl<'a> From<&'a SocketAddr> for SocketAddr {
    fn from(addr: &'a SocketAddr) -> Self {
        addr.to_owned()
    }
}

/// Specify a source address when connecting.
///
/// A `SrcAddr` differs from a `SocketAddr` since it allows an address with
/// with no port specified (an `Interface`).
///
/// A source address can be specified when a client communicate with a public
/// server from behind a private network. This allows the server's replies to
/// be routed properly.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::SrcAddr;
/// use std::convert::TryInto;
///
/// /// Specify an IPv4 addr with a dynamic port.
/// let src: SrcAddr = "192.168.1.17:*".try_into()?;
///
/// /// Specify a network interface.
/// let src: SrcAddr = "eth0".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SrcAddr {
    /// Bind to a socket address.
    Socket(SocketAddr),
    /// Bind to an interface.
    Interface(Interface),
}

impl FromStr for SrcAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if let Ok(addr) = SocketAddr::from_str(s) {
            Ok(SrcAddr::Socket(addr))
        } else {
            let host = Interface::from_str(s)?;
            Ok(SrcAddr::Interface(host))
        }
    }
}

tryfrom_fromstr!(SrcAddr);

impl fmt::Display for SrcAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SrcAddr::Socket(addr) => write!(f, "{}", addr),
            SrcAddr::Interface(host) => write!(f, "{}", host),
        }
    }
}

serde_display_tryfrom!(SrcAddr);

impl From<SocketAddr> for SrcAddr {
    fn from(addr: SocketAddr) -> Self {
        SrcAddr::Socket(addr)
    }
}

impl<'a> From<&'a SocketAddr> for SrcAddr {
    fn from(addr: &'a SocketAddr) -> Self {
        SrcAddr::Socket(addr.to_owned())
    }
}

impl<'a> From<&'a SrcAddr> for SrcAddr {
    fn from(src: &'a SrcAddr) -> Self {
        src.to_owned()
    }
}

/// A socket address with the `TCP` transport.
///
/// # Supported Sockets
/// [`Dish`], [`Radio`], [`Client`] and [`Server]
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::TcpAddr;
/// use std::convert::TryInto;
///
/// // Connecting using a IPv4 address and bind to `eth0` interface.
/// let ipv4: TcpAddr = "eth0;192.168.1.1:5555".try_into()?;
///
/// // Connecting using a IPv4 address.
/// let ipv6: TcpAddr = "[2001:db8::1]:8080".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TcpAddr {
    src: Option<SrcAddr>,
    addr: SocketAddr,
}

impl TcpAddr {
    pub fn new<A>(addr: A) -> Self
    where
        A: Into<SocketAddr>,
    {
        let addr = addr.into();
        Self { addr, src: None }
    }

    pub fn with_src<A, S>(addr: A, src: S) -> Self
    where
        A: Into<SocketAddr>,
        S: Into<SrcAddr>,
    {
        let addr = addr.into();
        let src = src.into();

        Self {
            addr,
            src: Some(src),
        }
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn src(&self) -> Option<&SrcAddr> {
        self.src.as_ref()
    }
}

impl FromStr for TcpAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if let Some(mid) = s.find(';') {
            let src = Some(SrcAddr::from_str(&s[..mid])?);
            let addr = SocketAddr::from_str(&s[mid + 1..])?;

            Ok(Self { src, addr })
        } else {
            let addr = SocketAddr::from_str(s)?;

            Ok(Self { src: None, addr })
        }
    }
}

tryfrom_fromstr!(TcpAddr);

impl fmt::Display for TcpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.src.is_some() {
            write!(f, "{};{}", self.addr, self.src.as_ref().unwrap())
        } else {
            write!(f, "{}", self.addr)
        }
    }
}

serde_display_tryfrom!(TcpAddr);

impl From<SocketAddr> for TcpAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { addr, src: None }
    }
}

impl IntoIterator for TcpAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a TcpAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

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

/// A socket address with the `UDP` transport.
///
/// # Supported Sockets
/// [`Dish`], [`Radio`]
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::UdpAddr;
/// use std::convert::TryInto;
///
/// // Multicast - UDP port 5555 on a Multicast address
/// let addr: UdpAddr = "239.0.0.1:5555".try_into()?;
///
/// // Same as above using IPv6 with joining only on interface eth0.
/// let addr: UdpAddr = "eth0;[ff02::1]:5555".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct UdpAddr {
    src: Option<SrcAddr>,
    addr: SocketAddr,
}

impl UdpAddr {
    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::addr::{UdpAddr, SocketAddr};
    /// use std::convert::TryInto;
    ///
    /// let addr: SocketAddr = "localhost:5555".try_into()?;
    /// // We can use a reference here which will allocate.
    /// let udp = UdpAddr::new(&addr);
    /// // We can also give ownership which does not allocate.
    /// let udp = UdpAddr::new(addr);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn new<A>(addr: A) -> Self
    where
        A: Into<SocketAddr>,
    {
        let addr = addr.into();
        Self { addr, src: None }
    }

    /// # Example
    /// ```
    /// # use failure::Error;
    /// #
    /// # fn main() -> Result<(), Error> {
    /// use libzmq::addr::{UdpAddr, SocketAddr, SrcAddr};
    /// use std::convert::TryInto;
    ///
    /// let addr: SocketAddr = "localhost:5555".try_into()?;
    /// let src: SrcAddr = "eth0".try_into()?;
    ///
    /// // We pass by reference which allocates, but we could
    /// // also give the ownership directly.
    /// let udp = UdpAddr::with_src(&addr, &src);
    ///
    /// // Note that `SocketAddr` implement `Into<SrcAddr>`,
    /// // so this is also valid.
    /// let udp = UdpAddr::with_src(&addr, &addr);
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    pub fn with_src<A, S>(addr: A, src: S) -> Self
    where
        A: Into<SocketAddr>,
        S: Into<SrcAddr>,
    {
        let addr = addr.into();
        let src = src.into();

        Self {
            addr,
            src: Some(src),
        }
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn src(&self) -> Option<&SrcAddr> {
        self.src.as_ref()
    }
}

impl FromStr for UdpAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if let Some(mid) = s.find(';') {
            let src = Some(SrcAddr::from_str(&s[..mid])?);
            let addr = SocketAddr::from_str(&s[mid + 1..])?;

            Ok(Self { src, addr })
        } else {
            let addr = SocketAddr::from_str(s)?;

            Ok(Self { src: None, addr })
        }
    }
}

tryfrom_fromstr!(UdpAddr);

impl fmt::Display for UdpAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.src.is_some() {
            write!(f, "{};{}", self.addr, self.src.as_ref().unwrap())
        } else {
            write!(f, "{}", self.addr)
        }
    }
}

serde_display_tryfrom!(UdpAddr);

impl From<SocketAddr> for UdpAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { addr, src: None }
    }
}

impl IntoIterator for UdpAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a UdpAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

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

/// A socket address with the `PGM` transport.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::PgmAddr;
/// use std::convert::TryInto;
///
/// // Connecting to the multicast address 239.192.1.1, port 5555,
/// // using the network interface with the address 192.168.1.1
/// // and the PGM protocol
/// let addr: PgmAddr = "192.168.1.1;239.192.1.1:5555".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
///
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PgmAddr {
    src: Option<SrcAddr>,
    addr: SocketAddr,
}

impl PgmAddr {
    pub fn new<A>(addr: A) -> Self
    where
        A: Into<SocketAddr>,
    {
        let addr = addr.into();
        Self { addr, src: None }
    }

    /// A source address can be specified when a client communicate with a public
    /// server from behind a private network. This allows the server's replies to
    /// be routed properly.
    pub fn with_src<A, S>(addr: A, src: S) -> Self
    where
        A: Into<SocketAddr>,
        S: Into<SrcAddr>,
    {
        let addr = addr.into();
        let src = src.into();

        Self {
            addr,
            src: Some(src),
        }
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn src(&self) -> Option<&SrcAddr> {
        self.src.as_ref()
    }
}

impl FromStr for PgmAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if let Some(mid) = s.find(';') {
            let src = Some(SrcAddr::from_str(&s[..mid])?);
            let addr = SocketAddr::from_str(&s[mid + 1..])?;

            Ok(Self { src, addr })
        } else {
            let addr = SocketAddr::from_str(s)?;

            Ok(Self { src: None, addr })
        }
    }
}

tryfrom_fromstr!(PgmAddr);

impl fmt::Display for PgmAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.src.is_some() {
            write!(f, "{};{}", self.addr, self.src.as_ref().unwrap())
        } else {
            write!(f, "{}", self.addr)
        }
    }
}

serde_display_tryfrom!(PgmAddr);

impl From<SocketAddr> for PgmAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { addr, src: None }
    }
}

impl IntoIterator for PgmAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a PgmAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

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

/// A socket address with the Encapsulated `PGM` transport.
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::EpgmAddr;
/// use std::convert::TryInto;
///
/// // Connecting to the multicast address 239.192.1.1, port 5555,
/// // using the first Ethernet network interface on Linux
/// // and the Encapsulated PGM protocol.
/// let addr: EpgmAddr = "eth0;239.192.1.1:5555".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EpgmAddr {
    src: Option<SrcAddr>,
    addr: SocketAddr,
}

impl EpgmAddr {
    pub fn new<A>(addr: A) -> Self
    where
        A: Into<SocketAddr>,
    {
        let addr = addr.into();
        Self { addr, src: None }
    }

    /// A source address can be specified when a client communicate with a public
    /// server from behind a private network. This allows the server's replies to
    /// be routed properly.
    pub fn with_src<A, S>(addr: A, src: S) -> Self
    where
        A: Into<SocketAddr>,
        S: Into<SrcAddr>,
    {
        let addr = addr.into();
        let src = src.into();

        Self {
            addr,
            src: Some(src),
        }
    }

    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn src(&self) -> Option<&SrcAddr> {
        self.src.as_ref()
    }
}

impl FromStr for EpgmAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if let Some(mid) = s.find(';') {
            let src = Some(SrcAddr::from_str(&s[..mid])?);
            let addr = SocketAddr::from_str(&s[mid + 1..])?;

            Ok(Self { src, addr })
        } else {
            let addr = SocketAddr::from_str(s)?;

            Ok(Self { src: None, addr })
        }
    }
}

tryfrom_fromstr!(EpgmAddr);

impl fmt::Display for EpgmAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.src.is_some() {
            write!(f, "{};{}", self.addr, self.src.as_ref().unwrap())
        } else {
            write!(f, "{}", self.addr)
        }
    }
}

serde_display_tryfrom!(EpgmAddr);

impl From<SocketAddr> for EpgmAddr {
    fn from(addr: SocketAddr) -> Self {
        Self { addr, src: None }
    }
}

impl IntoIterator for EpgmAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a EpgmAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

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

/// A socket address with inter-thread transport.
///
/// The `inproc` address is a non-empty `String` with at most
/// [`INPROC_MAX_SIZE`] characters.
///
/// The `inproc` transport can only be used by sockets that share the same `Ctx`.
///
/// # Supported Sockets
/// [`Dish`], [`Radio`], [`Client`] and [`Server]
///
/// # Example
/// ```
/// # use failure::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use libzmq::addr::InprocAddr;
/// use std::convert::TryInto;
///
/// // Can be any arbitrary string.
/// let test: InprocAddr = "test".try_into()?;
/// let rand: InprocAddr = "LKH*O&_[::O2134KG".try_into()?;
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`INPROC_MAX_SIZE`]: constant.INPROC_MAX_SIZE.html
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct InprocAddr {
    addr: String,
}

impl InprocAddr {
    pub fn new<S>(addr: S) -> Result<Self, AddrParseError>
    where
        S: Into<String>,
    {
        let addr = addr.into();

        if addr.is_empty() {
            Err(AddrParseError::new("empty addr"))
        } else if addr.len() > INPROC_MAX_SIZE {
            Err(AddrParseError::new(
                "addr cannot exceed `INPROC_MAX_SIZE` chars",
            ))
        } else {
            Ok(Self { addr })
        }
    }

    pub fn as_str(&self) -> &str {
        self.addr.as_str()
    }
}

impl FromStr for InprocAddr {
    type Err = AddrParseError;
    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        Self::new(s)
    }
}

impl fmt::Display for InprocAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.addr.fmt(f)
    }
}

serde_display_tryfrom!(InprocAddr);

impl TryFrom<String> for InprocAddr {
    type Error = AddrParseError;
    fn try_from(s: String) -> Result<Self, AddrParseError> {
        Self::new(s)
    }
}

impl<'a> TryFrom<&'a String> for InprocAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a String) -> Result<Self, AddrParseError> {
        Self::new(s.as_str())
    }
}

impl<'a> TryFrom<&'a str> for InprocAddr {
    type Error = AddrParseError;
    fn try_from(s: &'a str) -> Result<Self, AddrParseError> {
        Self::new(s)
    }
}

impl IntoIterator for InprocAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a InprocAddr {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

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
/// let endpoint: Endpoint = addr.into();
/// // A endpoint can be parsed directly from a string.
/// assert_eq!(endpoint, "tcp://127.0.0.1:9090".try_into().unwrap());
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
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
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
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Endpoint::Tcp(addr) => write!(f, "tcp://{}", addr),
            Endpoint::Inproc(addr) => write!(f, "inproc://{}", addr),
            Endpoint::Udp(addr) => write!(f, "udp://{}", addr),
            Endpoint::Epgm(addr) => write!(f, "epgm://{}", addr),
            Endpoint::Pgm(addr) => write!(f, "pgm://{}", addr),
        }
    }
}

impl FromStr for Endpoint {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, AddrParseError> {
        if let Some(index) = s.find("://") {
            match &s[0..index] {
                "tcp" => {
                    let addr = TcpAddr::from_str(&s[index + 3..])?;
                    Ok(Endpoint::Tcp(addr))
                }
                "inproc" => {
                    let addr = InprocAddr::from_str(&s[index + 3..])?;
                    Ok(Endpoint::Inproc(addr))
                }
                "udp" => {
                    let addr = UdpAddr::from_str(&s[index + 3..])?;
                    Ok(Endpoint::Udp(addr))
                }
                "pgm" => {
                    let addr = PgmAddr::from_str(&s[index + 3..])?;
                    Ok(Endpoint::Pgm(addr))
                }
                "epgm" => {
                    let addr = EpgmAddr::from_str(&s[index + 3..])?;
                    Ok(Endpoint::Epgm(addr))
                }
                _ => Err(AddrParseError::new("invalid transport")),
            }
        } else {
            Err(AddrParseError::new("invalid endpoint format"))
        }
    }
}

tryfrom_fromstr!(Endpoint);

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

serde_display_tryfrom!(Endpoint);

#[cfg(test)]
mod test {
    macro_rules! test_addr_ser_de {
        ($mod: ident, $name: ty, $string: expr) => {
            mod $mod {
                use crate::addr::*;
                use std::convert::TryInto;

                #[test]
                fn test_ser_de() {
                    let addr: $name = $string.try_into().unwrap();
                    let endpoint: Endpoint = addr.into();

                    let ron = ron::ser::to_string(&endpoint).unwrap();
                    println!("{}", ron);
                    let de: Endpoint = ron::de::from_str(&ron).unwrap();
                    assert_eq!(endpoint, de);
                }
            }
        };
    }

    test_addr_ser_de!(tcp, TcpAddr, "0.0.0.0:3000");
    test_addr_ser_de!(udp, UdpAddr, "0.0.0.0:3000");
    test_addr_ser_de!(pgm, PgmAddr, "0.0.0.0:3000");
    test_addr_ser_de!(epgm, EpgmAddr, "0.0.0.0:3000");
    test_addr_ser_de!(inproc, InprocAddr, "test");
}
