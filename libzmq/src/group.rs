use failure::Fail;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::{convert::TryFrom, fmt, str};

pub const MAX_GROUP_SIZE: usize = 15;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Fail, Hash)]
#[fail(display = "group cannot exceed 15 char")]
pub struct GroupError(());

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Group {
    inner: String,
}

impl Group {
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl Into<String> for Group {
    fn into(self) -> String {
        self.inner
    }
}

impl<'a> TryFrom<&'a str> for Group {
    type Error = GroupError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Group::try_from(value.to_owned())
    }
}

impl TryFrom<String> for Group {
    type Error = GroupError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() > MAX_GROUP_SIZE {
            Err(GroupError(()))
        } else {
            Ok(Group { inner: value })
        }
    }
}

impl<'a> TryFrom<&'a String> for Group {
    type Error = GroupError;
    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        Group::try_from(value.to_owned())
    }
}

impl str::FromStr for Group {
    type Err = GroupError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Group::try_from(s)
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
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
