use camino::Utf8PathBuf;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

use super::*;

/// [NormalizedPath] with extra constraint that it's absolute.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash, DeriveValueType)]
#[sea_orm(value_type = "String")]
#[serde(into = "String", try_from = "String")]
pub struct NormalizedPathAbsolute {
    path: Utf8PathBuf,
}

impl NormalizedPathAbsolute {
    pub fn to_path_buf(&self) -> Utf8PathBuf {
        self.path.clone()
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl std::fmt::Display for NormalizedPathAbsolute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl FromStr for NormalizedPathAbsolute {
    type Err = NormalizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = NormalizedPath::from_str(s)?;
        if !p.as_str().starts_with("/") {
            return Err(NormalizeError::new(
                p.to_path_buf(),
                NormalizeErrorKind::AbsoluteRequired,
            ));
        }

        Ok(Self {
            path: p.to_path_buf(),
        })
    }
}

impl TryFrom<String> for NormalizedPathAbsolute {
    type Error = NormalizeError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl TryFrom<Utf8PathBuf> for NormalizedPathAbsolute {
    type Error = NormalizeError;

    fn try_from(p: Utf8PathBuf) -> Result<Self, Self::Error> {
        Self::from_str(p.as_ref())
    }
}

impl From<NormalizedPathAbsolute> for String {
    fn from(p: NormalizedPathAbsolute) -> Self {
        p.to_string()
    }
}

impl Deref for NormalizedPathAbsolute {
    type Target = Utf8PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<str> for NormalizedPathAbsolute {
    fn as_ref(&self) -> &str {
        self.path.as_str()
    }
}

impl AsRef<Path> for NormalizedPathAbsolute {
    fn as_ref(&self) -> &Path {
        self.path.as_std_path()
    }
}
