//! Socket authentication and encryption.
//!
//! In *libzmq* each `Ctx` as a dedicated background
//! `AuthHandler` thread which will handle authentication and encryption
//! for all sockets within the same context.
//!
//! For two sockets to connect to
//! each other, they must have matching `Mechanism`. Then authentication is
//! performed depending on the configuration of the `AuthHandler`. This
//! configuration can be modified by using a `AuthClient` which send commands
//! to the handler.

pub(crate) mod client;
mod curve;
pub(crate) mod server;

pub use client::{AuthBuilder, AuthClient};
pub use curve::*;
pub use server::{StatusCode, StatusCodeParseError};

use crate::prelude::TryFrom;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::option;

/// Credentials for a `PLAIN` client.
/// # Example
/// ```
/// use libzmq::auth::*;
///
/// let creds = PlainClientCreds::new("user", "pass");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlainClientCreds {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl PlainClientCreds {
    /// Create a new `PlainClientCreds` from a username and password.
    pub fn new<U, P>(username: U, password: P) -> Self
    where
        U: Into<String>,
        P: Into<String>,
    {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Returns a reference to the username.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns a reference to the password.
    pub fn password(&self) -> &str {
        &self.password
    }
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
    /// A socket using the `Null` mechanism connect or accept connections from
    /// sockets also using the `Null` mechanism.
    Null,
    /// Plain text authentication with no encryption.
    ///
    /// A socket using the `PlainClient` mechanism connects to sockets using
    /// the `PlainServer` mechanism.
    PlainClient(PlainClientCreds),
    /// Plain text authentication with no encryption.
    ///
    /// A socket using the `PlainServer` mechanism accept connections from
    /// sockets using the `PlainClient` mechanism.
    PlainServer,
    /// Secure authentication and encryption using the `Curve` public-key
    /// mechanism.
    ///
    /// By default authentication is done using a whitelist of public keys.
    /// However, authentication can be disabled.
    ///
    /// A socket using the `CurveClient` mechanism connects to socket using the
    /// `CurveServer` mechanism.
    CurveClient(CurveClientCreds),
    /// Secure authentication and encryption using the `Curve` public-key
    /// mechanism.
    ///
    /// A socket using the `CurveServer` mechanism accepts connections from
    /// sockets using the `CurveClient` mechanism.
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

#[derive(Debug, Error)]
#[error("unsupported mechanism")]
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
