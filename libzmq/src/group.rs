//! Message groups used by the `Radio` and `Dish` sockets.

use failure::Fail;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{
    borrow::{Borrow, Cow, ToOwned},
    convert::TryFrom,
    ffi::{CStr, CString},
    fmt, ops, option, str,
};

/// The maximum allowed number of characters in a group.
pub const MAX_GROUP_SIZE: usize = 15;

/// An error returned when trying to parse a `GroupSlice` or `Group`.
///
/// This error occurs from a string that exceeds [`MAX_GROUP_SIZE`] char.
///
/// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
#[derive(Debug, Copy, Clone, PartialEq, Eq, Fail, Hash)]
#[fail(display = "unable to parse group: {}", msg)]
pub struct GroupParseError {
    msg: &'static str,
}

impl GroupParseError {
    fn new(msg: &'static str) -> Self {
        Self { msg }
    }
}

/// A slice to a [`Group`]
///
/// A `GroupSlice` cannot be constructed directly. It is either obtained from a
/// [`Group`] or from [`Msg::group`].
///
/// Namely, the length this group identifier must not exceed [`MAX_GROUP_SIZE`].
///
/// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
/// [`Group`]: struct.Group.html
/// [`Msg`]: struct.Msg.html#method.group
#[derive(PartialEq, Eq, Hash)]
pub struct GroupSlice {
    inner: CStr,
}

impl GroupSlice {
    pub(crate) fn from_c_str_unchecked(c_str: &CStr) -> &GroupSlice {
        unsafe { &*(c_str as *const CStr as *const GroupSlice) }
    }

    pub fn as_c_str(&self) -> &CStr {
        &self.inner
    }

    pub fn to_str(&self) -> Result<&str, str::Utf8Error> {
        self.inner.to_str()
    }

    pub fn to_string_lossy(&self) -> Cow<str> {
        self.inner.to_string_lossy()
    }

    pub fn to_bytes(&self) -> &[u8] {
        self.inner.to_bytes()
    }
}

impl fmt::Debug for GroupSlice {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, formatter)
    }
}

impl<'a> From<&'a Group> for &'a GroupSlice {
    fn from(s: &'a Group) -> Self {
        s.borrow()
    }
}

impl ToOwned for GroupSlice {
    type Owned = Group;

    fn to_owned(&self) -> Self::Owned {
        Group {
            inner: self.inner.to_owned(),
        }
    }
}

impl PartialEq<str> for GroupSlice {
    fn eq(&self, other: &str) -> bool {
        self.to_bytes() == other.as_bytes()
    }
}

impl PartialEq<GroupSlice> for str {
    fn eq(&self, other: &GroupSlice) -> bool {
        self.as_bytes() == other.to_bytes()
    }
}

impl PartialEq<Group> for GroupSlice {
    fn eq(&self, other: &Group) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl PartialEq<Group> for &GroupSlice {
    fn eq(&self, other: &Group) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl AsRef<CStr> for GroupSlice {
    fn as_ref(&self) -> &CStr {
        self.borrow()
    }
}

impl ops::Deref for GroupSlice {
    type Target = CStr;

    #[inline]
    fn deref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl<'a> fmt::Display for &'a GroupSlice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.inner.to_string_lossy())
    }
}

impl<'a> IntoIterator for &'a GroupSlice {
    type Item = &'a GroupSlice;
    type IntoIter = option::IntoIter<&'a GroupSlice>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

/// An `CString` that is a valid ØMQ group identifier.
///
/// Namely, the length this group identifier must not exceed [`MAX_GROUP_SIZE`].
///
/// # Example
/// ```
/// #
/// # use failure::Error;
/// # fn main() -> Result<(), Error> {
/// use libzmq::Group;
/// use std::convert::TryInto;
///
/// let string = "abc".to_owned();
///
/// let group: Group = string.try_into()?;
/// assert_eq!(group, "abc");
/// #
/// #     Ok(())
/// # }
/// ```
/// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Group {
    inner: CString,
}

impl Group {
    /// Creates a new `Group` from a container of bytes.
    ///
    /// A valid `Group` should not exceed [`MAX_GROUP_SIZE`] char and
    /// not contain any `nul` bytes.
    ///
    /// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
    pub fn new<B>(bytes: B) -> Result<Self, GroupParseError>
    where
        B: Into<Vec<u8>>,
    {
        let bytes = bytes.into();
        if bytes.len() > MAX_GROUP_SIZE {
            Err(GroupParseError::new("cannot exceed MAX_GROUP_SIZE char"))
        } else {
            let inner = CString::new(bytes)
                .map_err(|_| GroupParseError::new("cannot contain nul char"))?;
            Ok(Self { inner })
        }
    }

    pub fn as_c_str(&self) -> &CStr {
        self.inner.as_c_str()
    }

    pub fn to_str(&self) -> Result<&str, str::Utf8Error> {
        self.inner.to_str()
    }

    pub fn to_string_lossy(&self) -> Cow<str> {
        self.inner.to_string_lossy()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl<'a> From<&'a GroupSlice> for Group {
    fn from(s: &'a GroupSlice) -> Self {
        s.to_owned()
    }
}

impl From<Group> for CString {
    fn from(g: Group) -> CString {
        g.inner
    }
}

impl<'a> From<&'a Group> for Group {
    fn from(g: &'a Group) -> Group {
        g.to_owned()
    }
}

impl TryFrom<String> for Group {
    type Error = GroupParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> TryFrom<&'a String> for Group {
    type Error = GroupParseError;
    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Self::new(value.to_owned())
    }
}

impl<'a> TryFrom<&'a str> for Group {
    type Error = GroupParseError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value.to_owned())
    }
}

impl str::FromStr for Group {
    type Err = GroupParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.to_string_lossy())
    }
}

impl Borrow<GroupSlice> for Group {
    fn borrow(&self) -> &GroupSlice {
        GroupSlice::from_c_str_unchecked(self.inner.as_c_str())
    }
}

impl AsRef<GroupSlice> for Group {
    fn as_ref(&self) -> &GroupSlice {
        self.borrow()
    }
}

impl ops::Deref for Group {
    type Target = GroupSlice;

    #[inline]
    fn deref(&self) -> &GroupSlice {
        self.borrow()
    }
}

impl<'a> PartialEq<GroupSlice> for Group {
    fn eq(&self, other: &GroupSlice) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl<'a> PartialEq<&GroupSlice> for Group {
    fn eq(&self, other: &&GroupSlice) -> bool {
        self.as_c_str() == other.as_c_str()
    }
}

impl PartialEq<[u8]> for Group {
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

impl PartialEq<Group> for [u8] {
    fn eq(&self, other: &Group) -> bool {
        self == other.as_bytes()
    }
}

impl PartialEq<str> for Group {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<&str> for Group {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl PartialEq<Group> for str {
    fn eq(&self, other: &Group) -> bool {
        other.as_bytes() == self.as_bytes()
    }
}

impl IntoIterator for Group {
    type Item = Group;
    type IntoIter = option::IntoIter<Group>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a Group {
    type Item = &'a Group;
    type IntoIter = option::IntoIter<&'a Group>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl Serialize for Group {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_with::rust::display_fromstr::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for Group {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_with::rust::display_fromstr::deserialize(deserializer)
    }
}
