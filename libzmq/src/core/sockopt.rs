use crate::error::{msg_from_errno, Error, ErrorKind};
use libzmq_sys as sys;
use sys::errno;

use libc::{c_int, size_t};

use std::{
    ffi::CString,
    os::raw::c_void,
    time::Duration,
    {mem, ptr, str},
};

// This is the value `czmq` uses.
// https://github.com/zeromq/czmq/blob/master/src/zsock_option.inc#L389
const MAX_OPTION_SIZE: size_t = 255;

#[derive(Copy, Clone, Debug)]
pub(crate) enum SocketOption {
    Backlog = sys::ZMQ_BACKLOG as isize,
    ConnectTimeout = sys::ZMQ_CONNECT_TIMEOUT as isize,
    FileDescriptor = sys::ZMQ_FD as isize,
    HeartbeatInterval = sys::ZMQ_HEARTBEAT_IVL as isize,
    HeartbeatTimeout = sys::ZMQ_HEARTBEAT_TIMEOUT as isize,
    HeartbeatTtl = sys::ZMQ_HEARTBEAT_TTL as isize,
    SendHighWaterMark = sys::ZMQ_SNDHWM as isize,
    SendTimeout = sys::ZMQ_SNDTIMEO as isize,
    RecvHighWaterMark = sys::ZMQ_RCVHWM as isize,
    RecvTimeout = sys::ZMQ_RCVTIMEO as isize,
    NoDrop = sys::ZMQ_XPUB_NODROP as isize,
    Linger = sys::ZMQ_LINGER as isize,
    LastEndpoint = sys::ZMQ_LAST_ENDPOINT as isize,
    PlainPassword = sys::ZMQ_PLAIN_PASSWORD as isize,
    PlainUsername = sys::ZMQ_PLAIN_USERNAME as isize,
    PlainServer = sys::ZMQ_PLAIN_SERVER as isize,
    EnforceDomain = sys::ZMQ_ZAP_ENFORCE_DOMAIN as isize,
    ZapDomain = sys::ZMQ_ZAP_DOMAIN as isize,
    Subscribe = sys::ZMQ_SUBSCRIBE as isize,
    Unsubscribe = sys::ZMQ_UNSUBSCRIBE as isize,
    CurvePublicKey = sys::ZMQ_CURVE_PUBLICKEY as isize,
    CurveSecretKey = sys::ZMQ_CURVE_SECRETKEY as isize,
    CurveServer = sys::ZMQ_CURVE_SERVER as isize,
    CurveServerKey = sys::ZMQ_CURVE_SERVERKEY as isize,
}

impl From<SocketOption> for c_int {
    fn from(s: SocketOption) -> c_int {
        match s {
            SocketOption::Backlog => SocketOption::Backlog as c_int,
            SocketOption::ConnectTimeout => {
                SocketOption::ConnectTimeout as c_int
            }
            SocketOption::FileDescriptor => {
                SocketOption::FileDescriptor as c_int
            }
            SocketOption::HeartbeatInterval => {
                SocketOption::HeartbeatInterval as c_int
            }
            SocketOption::HeartbeatTimeout => {
                SocketOption::HeartbeatTimeout as c_int
            }
            SocketOption::HeartbeatTtl => SocketOption::HeartbeatTtl as c_int,
            SocketOption::SendHighWaterMark => {
                SocketOption::SendHighWaterMark as c_int
            }
            SocketOption::SendTimeout => SocketOption::SendTimeout as c_int,
            SocketOption::RecvHighWaterMark => {
                SocketOption::RecvHighWaterMark as c_int
            }
            SocketOption::RecvTimeout => SocketOption::RecvTimeout as c_int,
            SocketOption::NoDrop => SocketOption::NoDrop as c_int,
            SocketOption::Linger => SocketOption::Linger as c_int,
            SocketOption::LastEndpoint => SocketOption::LastEndpoint as c_int,
            SocketOption::PlainPassword => SocketOption::PlainPassword as c_int,
            SocketOption::PlainUsername => SocketOption::PlainUsername as c_int,
            SocketOption::PlainServer => SocketOption::PlainServer as c_int,
            SocketOption::EnforceDomain => SocketOption::EnforceDomain as c_int,
            SocketOption::ZapDomain => SocketOption::ZapDomain as c_int,
            SocketOption::Subscribe => SocketOption::Subscribe as c_int,
            SocketOption::Unsubscribe => SocketOption::Unsubscribe as c_int,
            SocketOption::CurvePublicKey => {
                SocketOption::CurvePublicKey as c_int
            }
            SocketOption::CurveSecretKey => {
                SocketOption::CurveSecretKey as c_int
            }
            SocketOption::CurveServer => SocketOption::CurveServer as c_int,
            SocketOption::CurveServerKey => {
                SocketOption::CurveServerKey as c_int
            }
        }
    }
}

fn getsockopt(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    mut_value_ptr: *mut c_void,
    size: &mut size_t,
) -> Result<(), Error> {
    let rc = unsafe {
        sys::zmq_getsockopt(mut_sock_ptr, option.into(), mut_value_ptr, size)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = match errno {
            errno::EINVAL => panic!("invalid option"),
            errno::ETERM => Error::new(ErrorKind::CtxTerminated),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EINTR => Error::new(ErrorKind::Interrupted),
            _ => panic!(msg_from_errno(errno)),
        };

        Err(err)
    } else {
        Ok(())
    }
}

pub(crate) fn getsockopt_bool(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<bool, Error> {
    let mut value = c_int::default();
    let mut size = mem::size_of::<c_int>();
    let value_ptr = &mut value as *mut c_int as *mut c_void;

    getsockopt(mut_sock_ptr, option, value_ptr, &mut size)?;

    Ok(value != 0)
}

pub(crate) fn getsockopt_scalar<T>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<T, Error>
where
    T: Default,
{
    let mut value = T::default();
    let mut size = mem::size_of::<T>();
    let value_ptr = &mut value as *mut T as *mut c_void;

    getsockopt(mut_sock_ptr, option, value_ptr, &mut size)?;

    Ok(value)
}

pub(crate) fn getsockopt_option_scalar<T>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    none_value: T,
) -> Result<Option<T>, Error>
where
    T: Default + Eq,
{
    let mut value = T::default();
    let mut size = mem::size_of::<T>();
    let value_ptr = &mut value as *mut T as *mut c_void;

    getsockopt(mut_sock_ptr, option, value_ptr, &mut size)?;

    if value == none_value {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub(crate) fn getsockopt_bytes(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<Option<Vec<u8>>, Error> {
    let mut size = MAX_OPTION_SIZE;
    let mut value = vec![0u8; size];
    let value_ptr = value.as_mut_ptr() as *mut c_void;

    getsockopt(mut_sock_ptr, option, value_ptr, &mut size)?;

    if size == 0 {
        Ok(None)
    } else {
        value.truncate(size);
        Ok(Some(value))
    }
}

pub(crate) fn getsockopt_string(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<Option<String>, Error> {
    match getsockopt_bytes(mut_sock_ptr, option)? {
        Some(mut bytes) => {
            // Remove null byte.
            bytes.pop();

            if bytes.is_empty() {
                Ok(None)
            } else {
                let c_str = unsafe { CString::from_vec_unchecked(bytes) };
                Ok(Some(c_str.into_string().unwrap()))
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn getsockopt_option_duration(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    none_value: i32,
) -> Result<Option<Duration>, Error> {
    let ms: i32 = getsockopt_scalar(mut_sock_ptr, option)?;
    if ms == none_value {
        Ok(None)
    } else {
        Ok(Some(Duration::from_millis(ms as u64)))
    }
}

fn setsockopt(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    value_ptr: *const c_void,
    size: size_t,
) -> Result<(), Error> {
    let rc = unsafe {
        sys::zmq_setsockopt(mut_sock_ptr, option.into(), value_ptr, size)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = match errno {
            errno::EINVAL => panic!("invalid option"),
            errno::ETERM => Error::new(ErrorKind::CtxTerminated),
            errno::ENOTSOCK => panic!("invalid socket"),
            errno::EINTR => Error::new(ErrorKind::Interrupted),
            _ => panic!(msg_from_errno(errno)),
        };

        Err(err)
    } else {
        Ok(())
    }
}

pub(crate) fn setsockopt_bool(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    value: bool,
) -> Result<(), Error> {
    let value = value as c_int;
    let size = mem::size_of::<c_int>() as size_t;
    let value_ptr = &value as *const c_int as *const c_void;

    setsockopt(mut_sock_ptr, option, value_ptr, size)
}

pub(crate) fn setsockopt_scalar<T>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    value: T,
) -> Result<(), Error> {
    let size = mem::size_of::<T>() as size_t;
    let value_ptr = &value as *const T as *const c_void;

    setsockopt(mut_sock_ptr, option, value_ptr, size)
}

pub(crate) fn setsockopt_option_scalar<T>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    maybe: Option<T>,
    none_value: T,
) -> Result<(), Error>
where
    T: Eq,
{
    let size = mem::size_of::<T>() as size_t;

    let value_ptr = match maybe {
        Some(value) => &value as *const T as *const c_void,
        None => &none_value as *const T as *const c_void,
    };
    setsockopt(mut_sock_ptr, option, value_ptr, size)
}

pub(crate) fn setsockopt_bytes(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    maybe: Option<&[u8]>,
) -> Result<(), Error> {
    match maybe {
        Some(bytes) => {
            let size = bytes.len();
            let value_ptr = bytes.as_ptr() as *const c_void;
            setsockopt(mut_sock_ptr, option, value_ptr, size)
        }
        None => setsockopt_null(mut_sock_ptr, option),
    }
}

pub(crate) fn setsockopt_str(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    maybe: Option<&str>,
) -> Result<(), Error> {
    // No need to add a terminating zero byte.
    // http://api.zeromq.org/master:zmq-setsockopt
    setsockopt_bytes(mut_sock_ptr, option, maybe.map(str::as_bytes))
}

pub(crate) fn setsockopt_null(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<(), Error> {
    setsockopt(mut_sock_ptr, option, ptr::null(), 0)
}

pub(crate) fn setsockopt_option_duration(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    maybe: Option<Duration>,
    none_value: i32,
) -> Result<(), Error> {
    if let Some(duration) = maybe {
        check_duration(duration)?;
    }

    setsockopt_option_scalar(
        mut_sock_ptr,
        option,
        maybe.map(|d| d.as_millis() as i32),
        none_value,
    )
}

pub(crate) fn setsockopt_duration(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    duration: Duration,
) -> Result<(), Error> {
    check_duration(duration)?;
    setsockopt_scalar(mut_sock_ptr, option, duration.as_millis() as i32)
}

fn check_duration(duration: Duration) -> Result<(), Error> {
    if duration.as_millis() > i32::max_value() as u128 {
        Err(Error::new(ErrorKind::InvalidInput(
            "ms in duration cannot be greater than i32::MAX",
        )))
    } else {
        Ok(())
    }
}
