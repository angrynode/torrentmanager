use camino::{Utf8Component as Component, Utf8PathBuf};
use sea_orm::*;
use serde::{Deserialize, Serialize};

use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

mod absolute;
mod component;
mod error;
mod relative;

pub use absolute::*;
pub use component::*;
pub use error::*;
pub use relative::*;

/// Path without special surprises:
///
/// - has to be valid UTF-8
/// - disallows parent dir traversal (`..`)
/// - has no trailing slash
/// - may contain current dir `./` but these will be discarded
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash, DeriveValueType)]
#[sea_orm(value_type = "String")]
#[serde(into = "String", try_from = "String")]
pub struct NormalizedPath {
    path: Utf8PathBuf,
}

impl NormalizedPath {
    pub fn to_path_buf(&self) -> Utf8PathBuf {
        self.path.clone()
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl Deref for NormalizedPath {
    type Target = Utf8PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<str> for NormalizedPath {
    fn as_ref(&self) -> &str {
        self.path.as_str()
    }
}

impl AsRef<Path> for NormalizedPath {
    fn as_ref(&self) -> &Path {
        self.path.as_std_path()
    }
}

impl std::fmt::Display for NormalizedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl FromStr for NormalizedPath {
    type Err = NormalizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let orig = Utf8PathBuf::from(s);
        let mut lexical = Utf8PathBuf::new();

        let mut iter = orig.components().peekable();
        let part = iter.peek();
        match part {
            Some(p @ Component::RootDir) | Some(p @ Component::CurDir) => {
                lexical.push(p);
                iter.next();
            }
            Some(Component::Prefix(prefix)) => {
                lexical.push(prefix.as_str());
                iter.next();
                if let Some(p @ Component::RootDir) = iter.peek() {
                    lexical.push(p);
                    iter.next();
                }
            }
            None => {
                return Err(NormalizeError::new(
                    orig,
                    NormalizeErrorKind::EmptyNotAllowed,
                ));
            }
            Some(Component::Normal(path)) => {
                lexical.push(path);
                iter.next();
            }
            Some(Component::ParentDir) => {
                return Err(NormalizeError::new(
                    orig,
                    NormalizeErrorKind::ParentNotAllowed,
                ));
            }
        }

        for component in iter {
            match component {
                Component::CurDir => {
                    return Err(NormalizeError::new(
                        orig,
                        NormalizeErrorKind::CurrentDirNotAllowed,
                    ));
                }
                Component::ParentDir => {
                    return Err(NormalizeError::new(
                        orig,
                        NormalizeErrorKind::ParentNotAllowed,
                    ));
                }
                Component::Normal(path) => lexical.push(path),
                // TODO: can we have a prefix here?
                _ => unreachable!(),
            }
        }

        Ok(NormalizedPath { path: lexical })
    }
}

impl TryFrom<String> for NormalizedPath {
    type Error = NormalizeError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl TryFrom<Utf8PathBuf> for NormalizedPath {
    type Error = NormalizeError;

    fn try_from(p: Utf8PathBuf) -> Result<Self, Self::Error> {
        Self::from_str(p.as_ref())
    }
}

impl From<NormalizedPath> for String {
    fn from(p: NormalizedPath) -> Self {
        p.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path() {
        let s = "foo/bar/baz";
        let p = NormalizedPathRelative::from_str(s).unwrap();
        assert_eq!(p.as_str(), s);
    }

    #[test]
    fn test_relative_path_absolute() {
        let s = "/foo/bar/baz";
        let p = NormalizedPathRelative::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::AbsoluteNotAllowed);
    }

    #[test]
    fn test_relative_path_parent() {
        let s = "foo/bar/../baz";
        let p = NormalizedPathRelative::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::ParentNotAllowed);
    }

    #[test]
    fn test_relative_path_current() {
        let s = "foo/bar/./baz";
        let p = Utf8PathBuf::from(s);
        println!("{:?}", p.components());
        let p = NormalizedPathRelative::from_str(s).unwrap();
        assert_eq!(p.as_str(), "foo/bar/baz");
    }

    #[test]
    fn test_relative_path_current_trailing() {
        let s = "foo/bar/./";
        let p = Utf8PathBuf::from(s);
        println!("{:?}", p.components());
        let p = NormalizedPathRelative::from_str(s).unwrap();
        assert_eq!(p.as_str(), "foo/bar");
    }

    #[test]
    fn test_relative_path_trailing() {
        let s = "foo/bar/baz/";
        let p = NormalizedPathRelative::from_str(s).unwrap();
        assert_eq!(p.as_str(), "foo/bar/baz");
    }

    #[test]
    fn test_absolute_path() {
        let s = "/foo/bar/baz";
        let p = NormalizedPathAbsolute::from_str(s).unwrap();
        assert_eq!(p.as_str(), s);
    }

    #[test]
    fn test_absolute_path_relative() {
        let s = "foo/bar/baz";
        let p = NormalizedPathAbsolute::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::AbsoluteRequired);
    }

    #[test]
    fn test_absolute_path_parent() {
        let s = "/foo/bar/../baz";
        let p = NormalizedPathAbsolute::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::ParentNotAllowed);
    }

    #[test]
    fn test_absolute_path_current() {
        let s = "/foo/bar/./baz";
        let p = NormalizedPathAbsolute::from_str(s).unwrap();
        assert_eq!(p.as_str(), "/foo/bar/baz");
    }

    #[test]
    fn test_absolute_path_current_trailing() {
        let s = "/foo/bar/./";
        let p = NormalizedPathAbsolute::from_str(s).unwrap();
        assert_eq!(p.as_str(), "/foo/bar");
    }

    #[test]
    fn test_absolute_path_trailing() {
        let s = "/foo/bar/baz/";
        let p = NormalizedPathAbsolute::from_str(s).unwrap();
        assert_eq!(p.as_str(), "/foo/bar/baz");
    }

    #[test]
    fn test_component_path() {
        let s = "foo";
        let p = NormalizedPathComponent::from_str(s).unwrap();
        assert_eq!(p.as_str(), s);
    }

    #[test]
    fn test_component_path_relative() {
        let s = "foo/bar/baz";
        let p = NormalizedPathComponent::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::SlashForbidden);
    }

    #[test]
    fn test_component_path_absolute() {
        let s = "/foo/bar/baz";
        let p = NormalizedPathComponent::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::SlashForbidden);
    }

    #[test]
    fn test_component_path_parent() {
        let s = "..";
        let p = NormalizedPathComponent::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::ParentNotAllowed);
    }

    #[test]
    fn test_component_path_current() {
        let s = ".";
        let p = NormalizedPathComponent::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::CurrentDirNotAllowed);
    }

    #[test]
    fn test_component_path_trailing() {
        let s = "foo/";
        let p = NormalizedPathComponent::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::SlashForbidden);
    }

    #[test]
    fn test_component_path_leading() {
        let s = "/foo";
        let p = NormalizedPathComponent::from_str(s).unwrap_err();
        assert_eq!(p.kind, NormalizeErrorKind::SlashForbidden);
    }

    #[test]
    fn test_path_deserialization() {
        let s = r#""/home/foo""#;
        let _p: NormalizedPath = serde_json::from_str(s).unwrap();
        let _p: NormalizedPathAbsolute = serde_json::from_str(s).unwrap();
        let p: Result<NormalizedPathRelative, _> = serde_json::from_str(s);
        assert!(p.is_err());
        let p: Result<NormalizedPathComponent, _> = serde_json::from_str(s);
        assert!(p.is_err());
    }
}
