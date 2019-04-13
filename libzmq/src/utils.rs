use libzmq_sys as sys;

use failure::{Backtrace, Context, Fail};

use std::{
    ffi,
    fmt::{self, Debug, Display},
    os::raw::*,
    str,
};

/// Reports the ØMQ library version.
///
/// Returns a tuple in the format `(Major, Minor, Patch)`.
///
/// See [`zmq_version`].
///
/// [`zmq_version`]: http://api.zeromq.org/4-2:zmq-version
///
/// ```
/// use libzmq::zmq_version;
///
/// assert_eq!(zmq_version(), (4, 3, 1));
/// ```
// This test acts as a canary when upgrading the libzmq
// version.
pub fn zmq_version() -> (i32, i32, i32) {
    let mut major = 0;
    let mut minor = 0;
    let mut patch = 0;
    unsafe {
        sys::zmq_version(
            &mut major as *mut c_int,
            &mut minor as *mut c_int,
            &mut patch as *mut c_int,
        );
    }
    (major, minor, patch)
}

/// Check for a ZMQ capability.
///
/// See [`zmq_has`].
///
/// [`zmq_has`]: http://api.zeromq.org/4-2:zmq-has
///
/// ```
/// use libzmq::zmq_has;
///
/// assert!(zmq_has("curve"));
/// ```
pub fn zmq_has(capability: &str) -> bool {
    let c_str = ffi::CString::new(capability).unwrap();
    unsafe { sys::zmq_has(c_str.as_ptr()) == 1 }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("../README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
