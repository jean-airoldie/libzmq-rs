use super::*;
use crate::*;

use failure::Fail;
use serde::{Deserialize, Serialize};

use std::{convert::TryFrom, option};

/// Credentials for a `PLAIN` client.
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
pub(crate) enum MechanismName {
    Null,
    Plain,
    Curve,
}

#[derive(Debug, Fail)]
#[fail(display = "unsupported mechanism")]
pub(crate) struct InvalidMechanismName;

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
