use crate::{
    core::{private, Socket},
    endpoint::Endpoint,
    error::Error,
};

use serde::{Deserialize, Serialize};

use std::time::Duration;

#[derive(Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[doc(hidden)]
pub struct SocketConfig {
    connect: Option<Vec<Endpoint>>,
    bind: Option<Vec<Endpoint>>,
    backlog: Option<i32>,
    connect_timeout: Option<Duration>,
    heartbeat_interval: Option<Duration>,
    heartbeat_timeout: Option<Duration>,
    heartbeat_ttl: Option<Duration>,
}

#[doc(hidden)]
pub trait AsSocketConfig: private::Sealed {
    fn as_socket_config(&self) -> &SocketConfig;

    fn as_mut_as_socket_config(&mut self) -> &mut SocketConfig;
}

/// The set of shared socket configuration methods.
pub trait SocketBuilder: AsSocketConfig {
    fn connect(&mut self, endpoints: Vec<Endpoint>) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.connect = Some(endpoints);
        self
    }

    fn bind(&mut self, endpoints: Vec<Endpoint>) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.bind = Some(endpoints);
        self
    }

    fn backlog(&mut self, len: i32) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.backlog = Some(len);
        self
    }

    fn connect_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.connect_timeout = maybe_duration;
        self
    }

    fn heartbeat_interval(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.heartbeat_interval = maybe_duration;
        self
    }

    fn heartbeat_timeout(
        &mut self,
        maybe_duration: Option<Duration>,
    ) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.heartbeat_timeout = maybe_duration;
        self
    }

    fn heartbeat_ttl(&mut self, maybe_duration: Option<Duration>) -> &mut Self {
        let mut config = self.as_mut_as_socket_config();
        config.heartbeat_ttl = maybe_duration;
        self
    }

    fn apply<S: Socket>(&self, socket: &S) -> Result<(), Error<()>> {
        let config = self.as_socket_config();

        if let Some(ref endpoints) = config.connect {
            for endpoint in endpoints {
                socket.connect(endpoint)?;
            }
        }
        if let Some(ref endpoints) = config.bind {
            for endpoint in endpoints {
                socket.bind(endpoint)?;
            }
        }
        if let Some(value) = config.backlog {
            socket.set_backlog(value)?;
        }
        socket.set_connect_timeout(config.connect_timeout)?;

        Ok(())
    }
}
