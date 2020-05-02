// The z85 codec logic is largely based on https://github.com/decafbad/z85
use super::Mechanism;
use crate::prelude::TryFrom;

use libzmq_sys as sys;
use thiserror::Error;

use byteorder::{BigEndian, ByteOrder};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use std::{ffi::CString, fmt, option, os::raw::c_char};

static LETTERS: [u8; 85] = [
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x61, 0x62,
    0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E,
    0x6F, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A,
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C,
    0x4D, 0x4E, 0x4F, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
    0x59, 0x5A, 0x2E, 0x2D, 0x3A, 0x2B, 0x3D, 0x5E, 0x21, 0x2F, 0x2A, 0x3F,
    0x26, 0x3C, 0x3E, 0x28, 0x29, 0x5B, 0x5D, 0x7B, 0x7D, 0x40, 0x25, 0x24,
    0x23,
];

static OCTETS: [u8; 96] = [
    0xFF, 0x44, 0xFF, 0x54, 0x53, 0x52, 0x48, 0xFF, 0x4B, 0x4C, 0x46, 0x41,
    0xFF, 0x3F, 0x3E, 0x45, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x40, 0xFF, 0x49, 0x42, 0x4A, 0x47, 0x51, 0x24, 0x25, 0x26,
    0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32,
    0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x4D,
    0xFF, 0x4E, 0x43, 0xFF, 0xFF, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
    0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
    0x1D, 0x1E, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x4F, 0xFF, 0x50, 0xFF, 0xFF,
];

// The size of a curve key in the z85 format.
const CURVE_CURVE_KEY_SIZE: usize = 40;

/// A error when encoding or decoding a `CurveKey`.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum CurveError {
    #[error("input string must have len of 40 char")]
    InvalidSize,
    #[error(
        "input string contains invalid byte 0x{:2X} at offset {}",
        byte,
        pos
    )]
    InvalidByte { pos: usize, byte: u8 },
}

fn z85_encode_chunk(input: &[u8]) -> [u8; 5] {
    let mut num = BigEndian::read_u32(input) as usize;
    let mut out = [0_u8; 5];

    for i in (0..5).rev() {
        out[i] = LETTERS[num % 85];
        num /= 85;
    }

    out
}

fn z85_encode(input: &[u8]) -> Result<String, CurveError> {
    let len = input.len();
    if len % 4 != 0 {
        panic!("input lenght must be div by 4");
    }

    let mut out = Vec::with_capacity(len / 4 * 5);
    for chunk in input.chunks(4) {
        out.extend_from_slice(&z85_encode_chunk(chunk));
    }

    unsafe { Ok(String::from_utf8_unchecked(out)) }
}

fn z85_decode_chunk(input: &[u8]) -> Result<[u8; 4], usize> {
    let mut num: u32 = 0;

    for (i, &byte) in input.iter().enumerate().take(5) {
        num *= 85;

        if byte < 0x20 || 0x7F < byte {
            return Err(i);
        }

        let b = OCTETS[byte as usize - 32];
        if b == 0xFF {
            return Err(i);
        }

        num += u32::from(b);
    }

    let mut out = [0_u8; 4];
    BigEndian::write_u32(&mut out, num);

    Ok(out)
}

fn z85_decode(input: &str) -> Result<Vec<u8>, CurveError> {
    let input = input.as_bytes();
    let len = input.len();
    if len % 5 != 0 {
        panic!("input length must be div by 5");
    }

    let mut out = Vec::with_capacity(len / 5 * 4);
    for (i, chunk) in input.chunks(5).enumerate() {
        match z85_decode_chunk(chunk) {
            Err(pos) => {
                return Err(CurveError::InvalidByte {
                    pos: i * 5 + pos,
                    byte: chunk[pos],
                });
            }
            Ok(out_chunk) => out.extend_from_slice(&out_chunk),
        }
    }

    Ok(out)
}

/// A public `CURVE` cryptographic key in the printable [`Z85`] representation.
///
/// # Example
/// ```
/// use libzmq::auth::*;
///
/// let cert = CurveCert::new_unique();
///
/// // Generate a public key from a curve certificate.
/// let public = cert.public().to_owned();
/// // Derive a public key from a secret key.
/// let derived: CurvePublicKey = cert.secret().into();
///
/// assert_eq!(public, derived);
/// ```
///
/// [`Z85`]: https://rfc.zeromq.org/spec:32/Z85/
/// [`CurveCert::new_unique()`]: struct.CurveCert.html#method.new_unique
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurvePublicKey {
    inner: CurveKey,
}

impl CurvePublicKey {
    /// Create a new `CurvePublicKey` from a valid `Z85` string.
    pub fn new<S>(text: S) -> Result<Self, CurveError>
    where
        S: Into<String>,
    {
        let inner = CurveKey::new(text)?;

        Ok(Self { inner })
    }

    /// Returns the key in `Z85` encoded string.
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl fmt::Display for CurvePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for CurvePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CurvePublicKey")
            .field("key", &self.as_str())
            .finish()
    }
}

impl From<CurveSecretKey> for CurvePublicKey {
    fn from(secret: CurveSecretKey) -> Self {
        let inner = CurveKey::from_secret(secret);

        Self { inner }
    }
}

impl<'a> From<&'a CurveSecretKey> for CurvePublicKey {
    fn from(secret: &'a CurveSecretKey) -> Self {
        let inner = CurveKey::from_secret(secret.to_owned());

        Self { inner }
    }
}

impl From<CurvePublicKey> for CurveKey {
    fn from(public: CurvePublicKey) -> Self {
        public.inner
    }
}

impl<'a> From<&'a CurvePublicKey> for CurvePublicKey {
    fn from(key: &'a CurvePublicKey) -> Self {
        key.to_owned()
    }
}

impl From<BinCurveKey> for CurvePublicKey {
    fn from(key: BinCurveKey) -> Self {
        let inner: CurveKey = key.into();

        Self { inner }
    }
}

impl<'a> From<&'a BinCurveKey> for CurvePublicKey {
    fn from(key: &'a BinCurveKey) -> Self {
        let inner: CurveKey = key.into();

        Self { inner }
    }
}

impl TryFrom<String> for CurvePublicKey {
    type Error = CurveError;
    fn try_from(text: String) -> Result<Self, CurveError> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a str> for CurvePublicKey {
    type Error = CurveError;
    fn try_from(text: &'a str) -> Result<Self, CurveError> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a String> for CurvePublicKey {
    type Error = CurveError;
    fn try_from(text: &'a String) -> Result<Self, CurveError> {
        Self::new(text.as_str())
    }
}

impl IntoIterator for CurvePublicKey {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a CurvePublicKey {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

/// A secret `CURVE` cryptographic key in the printable [`Z85`] representation.
///
/// Can be generated by [`CurveCert::new_unique()`].
///
/// [`Z85`]: https://rfc.zeromq.org/spec:32/Z85/
/// [`CurveCert::new_unique()`]: struct.CurveCert.html#method.new_unique
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurveSecretKey {
    inner: CurveKey,
}

impl CurveSecretKey {
    /// Create a new `CurveSecretKey` from a valid `Z85` string.
    pub fn new<S>(text: S) -> Result<Self, CurveError>
    where
        S: Into<String>,
    {
        let inner = CurveKey::new(text)?;

        Ok(Self { inner })
    }

    /// Returns the key in `Z85` encoded string.
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl fmt::Debug for CurveSecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CurveServerKey")
            .field("key", &self.as_str())
            .finish()
    }
}

impl From<CurveSecretKey> for CurveKey {
    fn from(public: CurveSecretKey) -> Self {
        public.inner
    }
}

impl<'a> From<&'a CurveSecretKey> for CurveSecretKey {
    fn from(key: &'a CurveSecretKey) -> Self {
        key.to_owned()
    }
}

impl From<BinCurveKey> for CurveSecretKey {
    fn from(key: BinCurveKey) -> Self {
        let inner: CurveKey = key.into();

        Self { inner }
    }
}

impl<'a> From<&'a BinCurveKey> for CurveSecretKey {
    fn from(key: &'a BinCurveKey) -> Self {
        let inner: CurveKey = key.into();

        Self { inner }
    }
}

impl TryFrom<String> for CurveSecretKey {
    type Error = CurveError;
    fn try_from(text: String) -> Result<Self, CurveError> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a str> for CurveSecretKey {
    type Error = CurveError;
    fn try_from(text: &'a str) -> Result<Self, CurveError> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a String> for CurveSecretKey {
    type Error = CurveError;
    fn try_from(text: &'a String) -> Result<Self, CurveError> {
        Self::new(text.as_str())
    }
}

impl IntoIterator for CurveSecretKey {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a CurveSecretKey {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CurveKey {
    text: String,
}

impl CurveKey {
    fn new<S>(text: S) -> Result<Self, CurveError>
    where
        S: Into<String>,
    {
        let text = text.into();
        if text.len() != CURVE_CURVE_KEY_SIZE {
            return Err(CurveError::InvalidSize);
        }

        let bytes = text.as_bytes();

        for (pos, &byte) in bytes.iter().enumerate() {
            if !LETTERS.contains(&byte) {
                return Err(CurveError::InvalidByte { pos, byte });
            }
        }

        Ok(Self { text })
    }

    fn from_secret<K>(secret: K) -> Self
    where
        K: Into<CurveKey>,
    {
        let secret = secret.into();
        let public = unsafe {
            CString::from_vec_unchecked(vec![0u8; CURVE_CURVE_KEY_SIZE])
        };
        let secret = unsafe { CString::from_vec_unchecked(secret.text.into()) };

        let rc = unsafe {
            sys::zmq_curve_public(
                public.as_ptr() as *mut c_char,
                secret.as_ptr() as *mut c_char,
            )
        };

        assert_eq!(rc, 0, "curve not supported");

        Self {
            text: public.into_string().unwrap(),
        }
    }

    fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

impl<'a> From<&'a CurveKey> for CurveKey {
    fn from(key: &'a CurveKey) -> Self {
        key.to_owned()
    }
}

impl TryFrom<String> for CurveKey {
    type Error = CurveError;
    fn try_from(text: String) -> Result<Self, CurveError> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a str> for CurveKey {
    type Error = CurveError;
    fn try_from(text: &'a str) -> Result<Self, CurveError> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a String> for CurveKey {
    type Error = CurveError;
    fn try_from(text: &'a String) -> Result<Self, CurveError> {
        Self::new(text.as_str())
    }
}

impl From<BinCurveKey> for CurveKey {
    fn from(key: BinCurveKey) -> Self {
        let text = z85_encode(key.as_bytes()).unwrap();

        // No need to validate.
        Self { text }
    }
}

impl<'a> From<&'a BinCurveKey> for CurveKey {
    fn from(key: &'a BinCurveKey) -> Self {
        let text = z85_encode(key.as_bytes()).unwrap();

        // No need to validate.
        Self { text }
    }
}

impl fmt::Display for CurveKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Serialize for CurveKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CurveKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TryFrom::try_from(s).map_err(de::Error::custom)
    }
}

impl IntoIterator for CurveKey {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a CurveKey {
    type Item = Self;
    type IntoIter = option::IntoIter<Self>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

/// A `CURVE` certificate containing a public and secret `CurveKey`.
///
/// # Example
/// ```
/// use libzmq::auth::CurveCert;
///
/// // Generate a new unique curve certificate.
/// let cert = CurveCert::new_unique();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveCert {
    public: CurvePublicKey,
    secret: CurveSecretKey,
}

impl CurveCert {
    /// Generate a new unique certificate.
    pub fn new_unique() -> Self {
        let public = unsafe {
            CString::from_vec_unchecked(vec![0u8; CURVE_CURVE_KEY_SIZE])
        };
        let secret = public.clone();

        let rc = unsafe {
            sys::zmq_curve_keypair(
                public.as_ptr() as *mut c_char,
                secret.as_ptr() as *mut c_char,
            )
        };

        assert_eq!(rc, 0, "curve not supported");

        // No need to check if returned z85 key is valid.
        let public = {
            let inner = CurveKey {
                text: public.into_string().unwrap(),
            };
            CurvePublicKey { inner }
        };
        let secret = {
            let inner = CurveKey {
                text: secret.into_string().unwrap(),
            };
            CurveSecretKey { inner }
        };

        Self { public, secret }
    }

    /// Returns a reference to the certificate's public key.
    pub fn public(&self) -> &CurvePublicKey {
        &self.public
    }

    /// Returns a reference to the certificate's secret key.
    pub fn secret(&self) -> &CurveSecretKey {
        &self.secret
    }
}

// Binary representation of the `CURVE` key. This is what is sent
// down the wire.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BinCurveKey {
    bytes: Vec<u8>,
}

impl BinCurveKey {
    pub(crate) fn new_unchecked(bytes: Vec<u8>) -> Self {
        BinCurveKey { bytes }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl From<CurveKey> for BinCurveKey {
    fn from(key: CurveKey) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        BinCurveKey { bytes }
    }
}

impl<'a> From<&'a CurveKey> for BinCurveKey {
    fn from(key: &'a CurveKey) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        BinCurveKey { bytes }
    }
}

impl From<CurvePublicKey> for BinCurveKey {
    fn from(key: CurvePublicKey) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        BinCurveKey { bytes }
    }
}

impl<'a> From<&'a CurvePublicKey> for BinCurveKey {
    fn from(key: &'a CurvePublicKey) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        BinCurveKey { bytes }
    }
}

impl From<CurveSecretKey> for BinCurveKey {
    fn from(key: CurveSecretKey) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        BinCurveKey { bytes }
    }
}

impl<'a> From<&'a CurveSecretKey> for BinCurveKey {
    fn from(key: &'a CurveSecretKey) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        BinCurveKey { bytes }
    }
}

/// Credentials for a `Curve` client.
///
/// # Example
/// ```
/// use libzmq::auth::*;
///
/// let server_cert = CurveCert::new_unique();
/// let client_cert = CurveCert::new_unique();
///
/// let creds = CurveClientCreds::new(server_cert.public())
///     .add_cert(client_cert);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveClientCreds {
    pub(crate) client: Option<CurveCert>,
    pub(crate) server: CurvePublicKey,
}

impl CurveClientCreds {
    /// Create a new `CurveClientCreds` from server's `CurvePublicKey`.
    pub fn new<S>(server: S) -> Self
    where
        S: Into<CurvePublicKey>,
    {
        Self {
            client: None,
            server: server.into(),
        }
    }

    /// Associates a client `CurveCert` with the credentials.
    pub fn add_cert<C>(mut self, client: C) -> Self
    where
        C: Into<CurveCert>,
    {
        self.client = Some(client.into());
        self
    }

    /// Returns a reference to the client certificate.
    pub fn cert(&self) -> Option<&CurveCert> {
        self.client.as_ref()
    }

    /// Returns a reference to the server public key.
    pub fn server(&self) -> &CurvePublicKey {
        &self.server
    }
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
/// # Example
/// ```
/// use libzmq::auth::*;
///
/// let server_cert = CurveCert::new_unique();
///
/// let creds = CurveServerCreds::new(server_cert.secret());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurveServerCreds {
    /// The server's `CurveSecretKey`.
    pub(crate) secret: CurveSecretKey,
}

impl CurveServerCreds {
    /// Create a new `CurveServerCreds` from a server secret `CurveSecretKey`.
    pub fn new<S>(secret: S) -> Self
    where
        S: Into<CurveSecretKey>,
    {
        Self {
            secret: secret.into(),
        }
    }

    /// Returns a reference to the server secret key.
    pub fn secret(&self) -> &CurveSecretKey {
        &self.secret
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

    const Z85_RFC: &str = "HelloWorld";
    const BIN_RFC: [u8; 8] = [0x86, 0x4F, 0xD2, 0x6F, 0xB5, 0x59, 0xF7, 0x5B];
    const CURVE_KEY_INVALID_BYTE: &str =
        "AAAAAAAAAAAAAAAAAAAA~AAAAAAAAAAAAAAAAAAA";
    const CURVE_KEY_SECRET: &str = "sqe2ZQ%<<?*(MV2Shf%9=CtldI@T^^pgrML1S.F/";
    const CURVE_KEY_PUBLIC: &str = "hb=GN9.(K*)]:{q*)XjsMgwfDTPJYh!w*n/xlIl+";

    #[test]
    fn curve_key_new_invalid_size() {
        let err = CurveKey::new(Z85_RFC).unwrap_err();
        assert_eq!(err, CurveError::InvalidSize);
    }

    #[test]
    fn curve_key_new_invalid_byte() {
        let err = CurveKey::new(CURVE_KEY_INVALID_BYTE).unwrap_err();
        assert_eq!(
            err,
            CurveError::InvalidByte {
                pos: 20,
                byte: 0x7E
            }
        );
    }

    #[test]
    fn curve_key_new() {
        CurveKey::new(CURVE_KEY_SECRET).unwrap();
    }

    #[test]
    fn curve_key_from_secret() {
        let secret = CurveKey::new(CURVE_KEY_SECRET).unwrap();
        let public = CurveKey::from_secret(&secret);
        assert_eq!(public.as_str(), CURVE_KEY_PUBLIC);
    }

    #[test]
    fn curve_cert_new_unique() {
        CurveCert::new_unique();
    }

    #[test]
    fn z85_encode_chunk_rfc() {
        let curve_chunk_1 = z85_decode_chunk(&Z85_RFC.as_bytes()[..5]).unwrap();
        let curve_chunk_2 = z85_decode_chunk(&Z85_RFC.as_bytes()[5..]).unwrap();
        assert_eq!(curve_chunk_1, BIN_RFC[..4]);
        assert_eq!(curve_chunk_2, BIN_RFC[4..]);
    }

    #[test]
    fn z85_decode_chunk_rfc() {
        let z85_chunk_1 = z85_encode_chunk(&BIN_RFC[..4]);
        let z85_chunk_2 = z85_encode_chunk(&BIN_RFC[4..]);
        assert_eq!(z85_chunk_1, Z85_RFC.as_bytes()[..5]);
        assert_eq!(z85_chunk_2, Z85_RFC.as_bytes()[5..]);
    }

    #[test]
    fn z85_encode_rfc() {
        let curve_key = z85_decode(&Z85_RFC).unwrap();
        assert_eq!(curve_key, BIN_RFC);
    }

    #[test]
    fn z85_decode_rfc() {
        let curve_key = z85_encode(&BIN_RFC).unwrap();
        assert_eq!(curve_key, Z85_RFC);
    }

    quickcheck! {
        fn codec_chunk_quickcheck(num: u32) -> bool {
            let mut buf = [0_u8; 4];
            BigEndian::write_u32(&mut buf, num);

            let z85_chunk = z85_encode_chunk(&buf);
            if let Ok(curve_chunk) = z85_decode_chunk(&z85_chunk) {
                if curve_chunk == buf {
                    return true;
                }
            }

            false
        }
    }

    quickcheck! {
        fn codec_quickcheck(input: Vec<u8>) -> bool {
            let mut input = input;
            input.extend_from_slice(&input.clone());
            input.extend_from_slice(&input.clone());

            if let Ok(z85) = z85_encode(&input) {
                if let Ok(curve) = z85_decode(&z85) {
                    return curve == input;
                }
            }

            false
        }
    }

    #[test]
    fn seven_bit_letters() {
        for &l in LETTERS.iter() {
            assert!(l < 0x80)
        }
    }
}
