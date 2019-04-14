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
            pub fn new() -> Result<Self, Error<()>> {
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
            pub fn with_ctx(ctx: crate::Ctx) -> Result<Self, crate::Error<()>> {
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

/// Implement the AsRawSocket trait.
macro_rules! impl_as_raw_socket_trait {
    ($name:ident) => {
        impl crate::core::AsRawSocket for $name {
            fn as_raw_socket(&self) -> *const std::os::raw::c_void {
                self.inner.socket
            }

            // This is safe since this socket is thread safe.
            fn as_mut_raw_socket(&self) -> *mut std::os::raw::c_void {
                self.inner.socket as *mut _
            }
        }
    };
}

macro_rules! impl_config_trait {
    ($name:ident) => {
        impl crate::core::AsSocketConfig for $name {
            #[doc(hidden)]
            fn as_socket_config(&self) -> &crate::core::SocketConfig {
                &self.inner
            }

            #[doc(hidden)]
            fn as_mut_as_socket_config(
                &mut self,
            ) -> &mut crate::core::SocketConfig {
                &mut self.inner
            }
        }

        impl crate::core::SocketBuilder for $name {}
    };
}
