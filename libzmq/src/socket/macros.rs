/// Implement the shared methods for a socket.
macro_rules! impl_socket_methods {
    ($name:ident, $sname:expr) => {
            /// Create a `
            #[doc = $sname]
            /// ` socket from the [`global context`]
            ///
            /// # Returned Error Variants
            /// * [`CtxTerminated`]
            /// * [`SocketLimit`]
            ///
            /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
            /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
            /// [`global context`]: ../ctx/struct.Ctx.html#method.global
            pub fn new() -> Result<Self, crate::error::Error<()>> {
                let inner = std::sync::Arc::new(
                    crate::core::RawSocket::new(
                        crate::core::RawSocketType::$name
                    )?
                );

                Ok(Self {
                    inner,
                })
            }

            /// Create a `
            #[doc = $sname]
            /// ` socket from a specific context.
            ///
            /// # Returned Error Variants
            /// * [`CtxTerminated`]
            /// * [`SocketLimit`]
            ///
            /// [`CtxTerminated`]: ../enum.ErrorKind.html#variant.CtxTerminated
            /// [`SocketLimit`]: ../enum.ErrorKind.html#variant.SocketLimit
            pub fn with_ctx(ctx: crate::Ctx) -> Result<Self, crate::error::Error<()>> {
                let inner = std::sync::Arc::new(
                    crate::core::RawSocket::with_ctx(
                        crate::core::RawSocketType::$name, ctx
                    )?
                );

                Ok(Self {
                    inner,
                })
            }

            /// Returns a reference to the context of the socket.
            pub fn ctx(&self) -> &crate::Ctx {
                &self.inner.ctx
            }

    };

    ($name:tt) => {
        impl_socket_methods!($name, stringify!($name));
    };
}

macro_rules! impl_get_raw_socket_trait {
    ($name:ident) => {
        impl crate::core::GetRawSocket for $name {
            fn raw_socket(&self) -> *const std::os::raw::c_void {
                self.inner.socket
            }

            // This is safe since this socket is thread safe.
            fn mut_raw_socket(&self) -> *mut std::os::raw::c_void {
                self.inner.socket as *mut _
            }
        }
    };
}

macro_rules! impl_get_socket_config_trait {
    ($name:ident) => {
        impl crate::core::GetSocketConfig for $name {
            #[doc(hidden)]
            fn socket_config(&self) -> &crate::core::SocketConfig {
                &self.socket_config
            }

            #[doc(hidden)]
            fn mut_socket_config(&mut self) -> &mut crate::core::SocketConfig {
                &mut self.socket_config
            }
        }
    };
}

macro_rules! impl_get_send_config_trait {
    ($name:ident) => {
        impl crate::core::GetSendConfig for $name {
            #[doc(hidden)]
            fn send_config(&self) -> &crate::core::SendConfig {
                &self.send_config
            }

            #[doc(hidden)]
            fn mut_send_config(&mut self) -> &mut crate::core::SendConfig {
                &mut self.send_config
            }
        }
    };
}

macro_rules! impl_get_recv_config_trait {
    ($name:ident) => {
        impl crate::core::GetRecvConfig for $name {
            #[doc(hidden)]
            fn recv_config(&self) -> &crate::core::RecvConfig {
                &self.recv_config
            }

            #[doc(hidden)]
            fn mut_recv_config(&mut self) -> &mut crate::core::RecvConfig {
                &mut self.recv_config
            }
        }
    };
}
