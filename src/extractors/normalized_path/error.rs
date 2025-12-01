use camino::Utf8PathBuf;
use snafu::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizeError {
    pub path: Utf8PathBuf,
    pub kind: NormalizeErrorKind,
}

impl NormalizeError {
    pub fn new(path: Utf8PathBuf, kind: NormalizeErrorKind) -> Self {
        Self { path, kind }
    }
}

impl std::error::Error for NormalizeError {}

#[derive(Clone, Debug, PartialEq, Snafu)]
pub enum NormalizeErrorKind {
    #[snafu(display("path cannot be empty"))]
    EmptyNotAllowed,
    #[snafu(display("`..` not allowed in path"))]
    ParentNotAllowed,
    #[snafu(display("`.` not allowed in path"))]
    CurrentDirNotAllowed,
    #[snafu(display("Path must relative"))]
    AbsoluteNotAllowed,
    #[snafu(display("Path must absolute"))]
    AbsoluteRequired,
    #[snafu(display("Component cannot contain slash"))]
    SlashForbidden,
}

impl std::fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.path, self.kind)
    }
}
