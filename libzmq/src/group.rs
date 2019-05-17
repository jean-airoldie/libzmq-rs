//! Message groups used by the `Radio` and `Dish` sockets.

use failure::Fail;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{
    borrow::{Borrow, ToOwned},
    convert::TryFrom,
    fmt, ops, option, str,
};

/// The maximum allowed number of characters in a group.
pub const MAX_GROUP_SIZE: usize = 15;

/// An error returned when trying to parse a `Group` or `GroupOwned`.
///
/// This error occurs from a string that exceeds [`MAX_GROUP_SIZE`] char.
///
/// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
#[derive(Debug, Copy, Clone, PartialEq, Eq, Fail, Hash)]
#[fail(display = "group cannot exceed MAX_GROUP_SIZE char")]
pub struct GroupParseError(());

/// A `str` slice that is a valid ØMQ group identifier.
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
/// let group: &Group = "some group".try_into()?;
///
/// let result: Result<&Group, _> = "group that exceed the char limit".try_into();
/// assert!(result.is_err());
/// #
/// #     Ok(())
/// # }
/// ```
///
/// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
#[derive(PartialEq, Eq, Hash)]
pub struct Group {
    inner: str,
}

impl Group {
    pub(crate) fn from_str_unchecked(s: &str) -> &Group {
        unsafe { &*(s as *const str as *const Group) }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl fmt::Debug for Group {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, formatter)
    }
}

impl<'a> From<&'a GroupOwned> for &'a Group {
    fn from(s: &'a GroupOwned) -> Self {
        s.borrow()
    }
}

impl ToOwned for Group {
    type Owned = GroupOwned;

    fn to_owned(&self) -> Self::Owned {
        GroupOwned {
            inner: self.inner.to_owned(),
        }
    }
}

impl<'a> TryFrom<&'a str> for &'a Group {
    type Error = GroupParseError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.len() > MAX_GROUP_SIZE {
            Err(GroupParseError(()))
        } else {
            Ok(Group::from_str_unchecked(value))
        }
    }
}

impl<'a> TryFrom<&'a String> for &'a Group {
    type Error = GroupParseError;
    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl PartialEq<str> for Group {
    fn eq(&self, other: &str) -> bool {
        *self == *Group::from_str_unchecked(other)
    }
}

impl PartialEq<Group> for str {
    fn eq(&self, other: &Group) -> bool {
        *other == *Group::from_str_unchecked(self)
    }
}

impl<'a> PartialEq<GroupOwned> for Group {
    fn eq(&self, other: &GroupOwned) -> bool {
        self.as_str() == other.as_str()
    }
}

impl AsRef<str> for Group {
    fn as_ref(&self) -> &str {
        self.borrow()
    }
}

impl ops::Deref for Group {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> fmt::Display for &'a Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

impl<'a> IntoIterator for &'a Group {
    type Item = &'a Group;
    type IntoIter = option::IntoIter<&'a Group>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

/// An owned `String` that is a valid ØMQ group identifier.
///
/// Namely, the length this group identifier must not exceed [`MAX_GROUP_SIZE`].
///
/// # Example
/// ```
/// #
/// # use failure::Error;
/// # fn main() -> Result<(), Error> {
/// use libzmq::GroupOwned;
/// use std::convert::TryInto;
///
/// let string = "abc".to_owned();
///
/// let group: GroupOwned = string.try_into()?;
/// assert_eq!(group.as_str(), "abc");
/// #
/// #     Ok(())
/// # }
/// ```
/// [`MAX_GROUP_SIZE`]: constant.MAX_GROUP_SIZE.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupOwned {
    inner: String,
}

impl<'a> From<&'a Group> for GroupOwned {
    fn from(s: &'a Group) -> Self {
        s.to_owned()
    }
}

impl From<GroupOwned> for String {
    fn from(g: GroupOwned) -> String {
        g.inner
    }
}

impl<'a> From<&'a GroupOwned> for GroupOwned {
    fn from(g: &'a GroupOwned) -> GroupOwned {
        g.to_owned()
    }
}

impl TryFrom<String> for GroupOwned {
    type Error = GroupParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_GROUP_SIZE {
            Err(GroupParseError(()))
        } else {
            Ok(Self { inner: value })
        }
    }
}

impl<'a> TryFrom<&'a String> for GroupOwned {
    type Error = GroupParseError;
    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        if value.len() > MAX_GROUP_SIZE {
            Err(GroupParseError(()))
        } else {
            Ok(Self {
                inner: value.to_owned(),
            })
        }
    }
}

impl<'a> TryFrom<&'a str> for GroupOwned {
    type Error = GroupParseError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.len() > MAX_GROUP_SIZE {
            Err(GroupParseError(()))
        } else {
            Ok(Self {
                inner: value.to_owned(),
            })
        }
    }
}

impl str::FromStr for GroupOwned {
    type Err = GroupParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl fmt::Display for GroupOwned {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Borrow<Group> for GroupOwned {
    fn borrow(&self) -> &Group {
        Group::from_str_unchecked(self.inner.as_str())
    }
}

impl AsRef<Group> for GroupOwned {
    fn as_ref(&self) -> &Group {
        self.borrow()
    }
}

impl ops::Deref for GroupOwned {
    type Target = Group;

    #[inline]
    fn deref(&self) -> &Group {
        self.borrow()
    }
}

impl<'a> PartialEq<Group> for GroupOwned {
    fn eq(&self, other: &Group) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for GroupOwned {
    fn eq(&self, other: &str) -> bool {
        &**self == other
    }
}

impl PartialEq<GroupOwned> for str {
    fn eq(&self, other: &GroupOwned) -> bool {
        &**other == self
    }
}

impl<'a> PartialEq<&'a str> for GroupOwned {
    fn eq(&self, other: &&'a str) -> bool {
        **self == **other
    }
}

impl<'a> PartialEq<GroupOwned> for &'a str {
    fn eq(&self, other: &GroupOwned) -> bool {
        **other == **self
    }
}

impl IntoIterator for GroupOwned {
    type Item = GroupOwned;
    type IntoIter = option::IntoIter<GroupOwned>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a GroupOwned {
    type Item = &'a GroupOwned;
    type IntoIter = option::IntoIter<&'a GroupOwned>;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl Serialize for GroupOwned {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_with::rust::display_fromstr::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for GroupOwned {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_with::rust::display_fromstr::deserialize(deserializer)
    }
}
