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
mod mechanism;
pub(crate) mod server;

pub use client::{AuthBuilder, AuthClient};
pub use curve::*;
pub use mechanism::*;
pub use server::{StatusCode, StatusCodeParseError};
