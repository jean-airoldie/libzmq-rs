use failure::Fail;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{borrow::Borrow, borrow::ToOwned, convert::TryFrom, fmt, ops, str};

pub const MAX_GROUP_SIZE: usize = 15;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Fail, Hash)]
#[fail(display = "group cannot exceed 15 char")]
pub struct GroupParseError(());

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

impl AsRef<Group> for Group {
    fn as_ref(&self) -> &Group {
        &self
    }
}

impl<'a> From<&'a GroupOwned> for &'a Group {
    fn from(s: &'a GroupOwned) -> Self {
        s.as_ref()
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

impl Borrow<Group> for GroupOwned {
    fn borrow(&self) -> &Group {
        Group::from_str_unchecked(self.as_str())
    }
}

impl AsRef<Group> for GroupOwned {
    fn as_ref(&self) -> &Group {
        &self.borrow()
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

impl ops::Deref for GroupOwned {
    type Target = Group;

    #[inline]
    fn deref(&self) -> &Group {
        &self.as_ref()
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
