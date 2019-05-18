// Largely based on https://github.com/decafbad/z85
use libzmq_sys as sys;

use byteorder::{BigEndian, ByteOrder};
use failure::Fail;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use std::{convert::TryFrom, ffi::CString, fmt, os::raw::c_char};

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

const Z85_KEY_SIZE: usize = 40;

#[derive(Debug, Fail, Eq, PartialEq)]
pub enum Z85Error {
    #[fail(display = "input string must have len of 40 char")]
    InvalidSize,
    #[fail(
        display = "input string contains invalid byte 0x{:2X} at offset {}",
        byte, pos
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

/// Encode arbitrary octets as base64.
fn z85_encode(input: &[u8]) -> Result<String, Z85Error> {
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

fn z85_decode(input: &str) -> Result<Vec<u8>, Z85Error> {
    let input = input.as_bytes();
    let len = input.len();
    if len % 5 != 0 {
        panic!("input length must be div by 5");
    }

    let mut out = Vec::with_capacity(len / 5 * 4);
    for (i, chunk) in input.chunks(5).enumerate() {
        match z85_decode_chunk(chunk) {
            Err(pos) => {
                return Err(Z85Error::InvalidByte {
                    pos: i * 5 + pos,
                    byte: chunk[pos],
                });
            }
            Ok(out_chunk) => out.extend_from_slice(&out_chunk),
        }
    }

    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Z85Key {
    text: String,
}

impl Z85Key {
    pub fn new<S>(text: S) -> Result<Self, Z85Error>
    where
        S: Into<String>,
    {
        let text = text.into();
        if text.len() != Z85_KEY_SIZE {
            return Err(Z85Error::InvalidSize);
        }

        let bytes = text.as_bytes();

        for (pos, &byte) in bytes.iter().enumerate() {
            if !LETTERS.contains(&byte) {
                return Err(Z85Error::InvalidByte { pos, byte });
            }
        }

        Ok(Self { text })
    }

    /// Derive a public key from a secret key.
    pub fn from_secret<K>(secret: K) -> Self
    where
        K: Into<Z85Key>,
    {
        let secret = secret.into();
        let public =
            unsafe { CString::from_vec_unchecked(vec![0u8; Z85_KEY_SIZE]) };
        let secret = unsafe { CString::from_vec_unchecked(secret.text.into()) };

        let rc = unsafe {
            sys::zmq_curve_public(
                public.as_ptr() as *mut c_char,
                secret.as_ptr() as *mut c_char,
            )
        };

        assert_eq!(rc, 0, "curve not supported");

        Z85Key {
            text: public.into_string().unwrap(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

impl<'a> From<&'a Z85Key> for Z85Key {
    fn from(key: &'a Z85Key) -> Self {
        key.to_owned()
    }
}

impl TryFrom<String> for Z85Key {
    type Error = Z85Error;
    fn try_from(text: String) -> Result<Self, Z85Error> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a str> for Z85Key {
    type Error = Z85Error;
    fn try_from(text: &'a str) -> Result<Self, Z85Error> {
        Self::new(text)
    }
}

impl<'a> TryFrom<&'a String> for Z85Key {
    type Error = Z85Error;
    fn try_from(text: &'a String) -> Result<Self, Z85Error> {
        Self::new(text.as_str())
    }
}

impl From<CurveKey> for Z85Key {
    fn from(key: CurveKey) -> Self {
        let text = z85_encode(key.as_bytes()).unwrap();

        // No need to validate.
        Self { text }
    }
}

impl<'a> From<&'a CurveKey> for Z85Key {
    fn from(key: &'a CurveKey) -> Self {
        let text = z85_encode(key.as_bytes()).unwrap();

        // No need to validate.
        Self { text }
    }
}

impl fmt::Display for Z85Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl Serialize for Z85Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Z85Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TryFrom::try_from(s).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Z85Cert {
    public: Z85Key,
    secret: Z85Key,
}

impl Z85Cert {
    pub fn new_unique() -> Self {
        let public =
            unsafe { CString::from_vec_unchecked(vec![0u8; Z85_KEY_SIZE]) };
        let secret = public.clone();

        let rc = unsafe {
            sys::zmq_curve_keypair(
                public.as_ptr() as *mut c_char,
                secret.as_ptr() as *mut c_char,
            )
        };

        assert_eq!(rc, 0, "curve not supported");

        // No need to check if returned z85 key is valid.
        let public = Z85Key {
            text: public.into_string().unwrap(),
        };
        let secret = Z85Key {
            text: secret.into_string().unwrap(),
        };

        Self { public, secret }
    }

    pub fn public(&self) -> &Z85Key {
        &self.public
    }

    pub fn secret(&self) -> &Z85Key {
        &self.secret
    }
}

impl From<CurveCert> for Z85Cert {
    fn from(cert: CurveCert) -> Self {
        let public: Z85Key = cert.public.into();
        let secret: Z85Key = cert.secret.into();

        Self { public, secret }
    }
}

impl<'a> From<&'a CurveCert> for Z85Cert {
    fn from(cert: &'a CurveCert) -> Self {
        let public: Z85Key = cert.public().into();
        let secret: Z85Key = cert.secret().into();

        Self { public, secret }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CurveKey {
    bytes: Vec<u8>,
}

impl CurveKey {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl From<Z85Key> for CurveKey {
    fn from(key: Z85Key) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        Self { bytes }
    }
}

impl<'a> From<&'a Z85Key> for CurveKey {
    fn from(key: &'a Z85Key) -> Self {
        let bytes = z85_decode(key.as_str()).unwrap();

        Self { bytes }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct CurveCert {
    public: CurveKey,
    secret: CurveKey,
}

impl CurveCert {
    pub fn public(&self) -> &CurveKey {
        &self.public
    }

    pub fn secret(&self) -> &CurveKey {
        &self.secret
    }
}

impl From<Z85Cert> for CurveCert {
    fn from(pair: Z85Cert) -> Self {
        let public: CurveKey = pair.public.into();
        let secret: CurveKey = pair.secret.into();

        Self { public, secret }
    }
}

impl<'a> From<&'a Z85Cert> for CurveCert {
    fn from(pair: &'a Z85Cert) -> Self {
        let public: CurveKey = pair.public().into();
        let secret: CurveKey = pair.secret().into();

        Self { public, secret }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::quickcheck;

    const Z85_RFC: &str = "HelloWorld";
    const CURVE_RFC: [u8; 8] = [0x86, 0x4F, 0xD2, 0x6F, 0xB5, 0x59, 0xF7, 0x5B];
    const Z85_KEY_INVALID_BYTE: &str =
        "AAAAAAAAAAAAAAAAAAAA~AAAAAAAAAAAAAAAAAAA";
    const Z85_KEY_SECRET: &str = "sqe2ZQ%<<?*(MV2Shf%9=CtldI@T^^pgrML1S.F/";
    const Z85_KEY_PUBLIC: &str = "hb=GN9.(K*)]:{q*)XjsMgwfDTPJYh!w*n/xlIl+";

    #[test]
    fn z85_key_new_invalid_size() {
        let err = Z85Key::new(Z85_RFC).unwrap_err();
        assert_eq!(err, Z85Error::InvalidSize);
    }

    #[test]
    fn z85_key_new_invalid_byte() {
        let err = Z85Key::new(Z85_KEY_INVALID_BYTE).unwrap_err();
        assert_eq!(
            err,
            Z85Error::InvalidByte {
                pos: 20,
                byte: 0x7E
            }
        );
    }

    #[test]
    fn z85_key_new() {
        Z85Key::new(Z85_KEY_SECRET).unwrap();
    }

    #[test]
    fn z85_key_from_secret() {
        let secret = Z85Key::new(Z85_KEY_SECRET).unwrap();
        let public = Z85Key::from_secret(&secret);
        assert_eq!(public.as_str(), Z85_KEY_PUBLIC);
    }

    #[test]
    fn z85_cert_new_unique() {
        Z85Cert::new_unique();
    }

    #[test]
    fn z85_encode_chunk_rfc() {
        let curve_chunk_1 = z85_decode_chunk(&Z85_RFC.as_bytes()[..5]).unwrap();
        let curve_chunk_2 = z85_decode_chunk(&Z85_RFC.as_bytes()[5..]).unwrap();
        assert_eq!(curve_chunk_1, CURVE_RFC[..4]);
        assert_eq!(curve_chunk_2, CURVE_RFC[4..]);
    }

    #[test]
    fn z85_decode_chunk_rfc() {
        let z85_chunk_1 = z85_encode_chunk(&CURVE_RFC[..4]);
        let z85_chunk_2 = z85_encode_chunk(&CURVE_RFC[4..]);
        assert_eq!(z85_chunk_1, Z85_RFC.as_bytes()[..5]);
        assert_eq!(z85_chunk_2, Z85_RFC.as_bytes()[5..]);
    }

    #[test]
    fn z85_encode_rfc() {
        let curve_key = z85_decode(&Z85_RFC).unwrap();
        assert_eq!(curve_key, CURVE_RFC);
    }

    #[test]
    fn z85_decode_rfc() {
        let z85_key = z85_encode(&CURVE_RFC).unwrap();
        assert_eq!(z85_key, Z85_RFC);
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
            let mut input = input.clone();
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
