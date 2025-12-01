use camino::Utf8PathBuf;
use sea_orm::*;
use serde::{Deserialize, Serialize};

use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

use super::*;

/// [NormalizedPath] with extra constraint that it contains no slashes.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Hash, DeriveValueType)]
#[sea_orm(value_type = "String")]
#[serde(into = "String", try_from = "String")]
pub struct NormalizedPathComponent {
    path: Utf8PathBuf,
}

impl NormalizedPathComponent {
    pub fn to_path_buf(&self) -> Utf8PathBuf {
        self.path.clone()
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl std::fmt::Display for NormalizedPathComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl FromStr for NormalizedPathComponent {
    type Err = NormalizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains("/") {
            return Err(NormalizeError::new(
                Utf8PathBuf::from(s),
                NormalizeErrorKind::SlashForbidden,
            ));
        }

        if s == "." {
            return Err(NormalizeError::new(
                Utf8PathBuf::from(s),
                NormalizeErrorKind::CurrentDirNotAllowed,
            ));
        }

        let p = NormalizedPath::from_str(s)?;
        Ok(Self {
            path: p.to_path_buf(),
        })
    }
}

impl TryFrom<String> for NormalizedPathComponent {
    type Error = NormalizeError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl From<NormalizedPathComponent> for String {
    fn from(p: NormalizedPathComponent) -> Self {
        p.to_string()
    }
}

impl Deref for NormalizedPathComponent {
    type Target = Utf8PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<str> for NormalizedPathComponent {
    fn as_ref(&self) -> &str {
        self.path.as_str()
    }
}

impl AsRef<Path> for NormalizedPathComponent {
    fn as_ref(&self) -> &Path {
        self.path.as_std_path()
    }
}
