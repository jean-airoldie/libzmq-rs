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
    Backlog,
    ConnectTimeout,
    FileDescriptor,
    HeartbeatInterval,
    HeartbeatTimeout,
    HeartbeatTtl,
    SendHighWaterMark,
    SendTimeout,
    RecvHighWaterMark,
    RecvTimeout,
    NoDrop,
    Linger,
}

impl Into<c_int> for SocketOption {
    fn into(self) -> c_int {
        match self {
            SocketOption::Backlog => sys::ZMQ_BACKLOG as c_int,
            SocketOption::ConnectTimeout => sys::ZMQ_CONNECT_TIMEOUT as c_int,
            SocketOption::FileDescriptor => sys::ZMQ_FD as c_int,
            SocketOption::HeartbeatInterval => sys::ZMQ_HEARTBEAT_IVL as c_int,
            SocketOption::HeartbeatTimeout => {
                sys::ZMQ_HEARTBEAT_TIMEOUT as c_int
            }
            SocketOption::HeartbeatTtl => sys::ZMQ_HEARTBEAT_TTL as c_int,
            SocketOption::SendHighWaterMark => sys::ZMQ_SNDHWM as c_int,
            SocketOption::SendTimeout => sys::ZMQ_SNDTIMEO as c_int,
            SocketOption::RecvHighWaterMark => sys::ZMQ_RCVHWM as c_int,
            SocketOption::RecvTimeout => sys::ZMQ_RCVTIMEO as c_int,
            SocketOption::NoDrop => sys::ZMQ_XPUB_NODROP as c_int,
            SocketOption::Linger => sys::ZMQ_LINGER as c_int,
        }
    }
}

fn getsockopt(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    mut_value_ptr: *mut c_void,
    size: &mut size_t,
) -> Result<(), Error<()>> {
    let rc = unsafe {
        sys::zmq_getsockopt(mut_sock_ptr, option.into(), mut_value_ptr, size)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => panic!("invalid option"),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                _ => panic!(msg_from_errno(errno)),
            }
        };

        Err(err)
    } else {
        Ok(())
    }
}

pub(crate) fn getsockopt_bool(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<bool, Error<()>> {
    let mut value = c_int::default();
    let mut size = mem::size_of::<c_int>();
    let value_ptr = &mut value as *mut c_int as *mut c_void;

    getsockopt(mut_sock_ptr, option, value_ptr, &mut size)?;

    Ok(value != 0)
}

pub(crate) fn getsockopt_scalar<T>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<T, Error<()>>
where
    T: Default,
{
    let mut value = T::default();
    let mut size = mem::size_of::<T>();
    let value_ptr = &mut value as *mut T as *mut c_void;

    getsockopt(mut_sock_ptr, option, value_ptr, &mut size)?;

    Ok(value)
}

pub(crate) fn getsockopt_bytes(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<Option<Vec<u8>>, Error<()>> {
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
) -> Result<Option<String>, Error<()>> {
    match getsockopt_bytes(mut_sock_ptr, option)? {
        Some(bytes) => {
            let c_str = unsafe { CString::from_vec_unchecked(bytes) };
            Ok(Some(c_str.into_string().unwrap()))
        }
        None => Ok(None),
    }
}

pub(crate) fn getsockopt_duration(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    none_value: i32,
) -> Result<Option<Duration>, Error<()>> {
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
) -> Result<(), Error<()>> {
    let rc = unsafe {
        sys::zmq_setsockopt(mut_sock_ptr, option.into(), value_ptr, size)
    };

    if rc == -1 {
        let errno = unsafe { sys::zmq_errno() };
        let err = {
            match errno {
                errno::EINVAL => panic!("invalid option"),
                errno::ETERM => Error::new(ErrorKind::CtxTerminated),
                errno::ENOTSOCK => panic!("invalid socket"),
                errno::EINTR => Error::new(ErrorKind::Interrupted),
                _ => panic!(msg_from_errno(errno)),
            }
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
) -> Result<(), Error<()>> {
    let value = value as c_int;
    let size = mem::size_of::<c_int>() as size_t;
    let value_ptr = &value as *const c_int as *const c_void;

    setsockopt(mut_sock_ptr, option, value_ptr, size)
}

pub(crate) fn setsockopt_scalar<T>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    value: T,
) -> Result<(), Error<()>> {
    let size = mem::size_of::<T>() as size_t;
    let value_ptr = &value as *const T as *const c_void;

    setsockopt(mut_sock_ptr, option, value_ptr, size)
}

pub(crate) fn setsockopt_bytes(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    bytes: &[u8],
) -> Result<(), Error<()>> {
    let size = bytes.len();
    let value_ptr = bytes.as_ptr() as *const c_void;

    setsockopt(mut_sock_ptr, option, value_ptr, size)
}

pub(crate) fn setsockopt_str<S>(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    maybe_string: Option<S>,
) -> Result<(), Error<()>>
where
    S: AsRef<str>,
{
    match maybe_string {
        Some(string) => {
            // No need to add a terminating zero byte.
            // http://api.zeromq.org/master:zmq-setsockopt
            setsockopt_bytes(mut_sock_ptr, option, string.as_ref().as_bytes())
        }
        None => setsockopt_null(mut_sock_ptr, option),
    }
}

pub(crate) fn setsockopt_null(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
) -> Result<(), Error<()>> {
    setsockopt(mut_sock_ptr, option, ptr::null(), 0)
}

pub(crate) fn setsockopt_duration(
    mut_sock_ptr: *mut c_void,
    option: SocketOption,
    maybe_duration: Option<Duration>,
    none_value: i32,
) -> Result<(), Error<()>> {
    match maybe_duration {
        Some(duration) => {
            let ms = checked_duration_ms(duration)?;
            setsockopt_scalar(mut_sock_ptr, option, ms)
        }
        None => setsockopt_scalar(mut_sock_ptr, option, none_value),
    }
}

fn checked_duration_ms(duration: Duration) -> Result<i32, Error<()>> {
    if duration.as_millis() > i32::max_value() as u128 {
        Err(Error::new(ErrorKind::InvalidInput {
            msg: "ms in duration cannot be greater than i32::MAX",
        }))
    } else if duration.as_millis() == 0 {
        Err(Error::new(ErrorKind::InvalidInput {
            msg: "ms in duration cannot be zero",
        }))
    } else {
        Ok(duration.as_millis() as i32)
    }
}
