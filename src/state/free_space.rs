// Much of this file is taken from the uutils coreutils package,
// distributed under the MIT License:
//
// For the full copyright and license information, please view the LICENSE
// file that was distributed there:
// https://github.com/uutils/coreutils/blob/main/LICENSE

use camino::{Utf8Path, Utf8PathBuf};
use snafu::prelude::*;
use uucore::fsext::{FsUsage, read_fs_list, statfs};

use std::ffi::OsString;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum FreeSpaceError {
    #[snafu(display("Failed to resolve provided path {path}"))]
    ResolvePath {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Failed to read partitions from system"))]
    Partitions { reason: String },
    #[snafu(display("Failed to read info about specific partition"))]
    Partition { reason: String },
}

/// Remaining space on a partition.
///
/// Uses (vendored) uu_df from uutils under the hood.
#[derive(Clone, Debug)]
pub struct FreeSpace {
    /// Number of remaining GiB.
    pub free_space_gib: u64,
    /// Number of total GiB.
    total_space_gib: u64,
    /// Percentage of remaining available space.
    pub free_space_percent: u64,
}

impl FreeSpace {
    pub fn from_path(path: &Utf8Path) -> Result<Self, FreeSpaceError> {
        let path = path.canonicalize().context(ResolvePathSnafu {
            path: path.to_path_buf(),
        })?;

        // Copied from uutils df package (MIT license)
        let mounts: Vec<_> = read_fs_list().map_err(|e| FreeSpaceError::Partitions {
            reason: e.to_string(),
        })?;
        let maybe_mount_point = mounts
            .iter()
            .map(|m| (m, std::fs::canonicalize(&m.dev_name)))
            .filter(|m| m.1.is_ok())
            .map(|m| (m.0, m.1.ok().unwrap()))
            .find(|m| m.1.eq(&path))
            .map(|m| m.0);
        let mount_info = maybe_mount_point
            .or_else(|| {
                mounts
                    .iter()
                    .filter(|mi| path.starts_with(&mi.mount_dir))
                    .max_by_key(|mi| mi.mount_dir.len())
            })
            .unwrap();
        let stat_path = if mount_info.mount_dir.is_empty() {
            OsString::from(mount_info.dev_name.clone())
        } else {
            mount_info.mount_dir.clone()
        };
        let usage = FsUsage::new(statfs(&stat_path).map_err(|e| FreeSpaceError::Partition {
            reason: e.to_string(),
        })?);

        // Calculate used/free space on partition
        let blocks_used = usage.blocks.saturating_sub(usage.bfree);
        let bytes_free = usage.blocksize * usage.bavail;

        let percent_used = if usage.blocks == 0 {
            0.0
        } else {
            blocks_used as f64 / (blocks_used + usage.bavail) as f64
        } * 100.0;
        // Round up to the higher percent
        let percent_used = percent_used.ceil();

        let gib_free = bytes_free as f64 / 1024.0 / 1024.0 / 1024.0;
        // Round down to the lower GiB
        let gib_free = gib_free.floor();

        let gib_total = (usage.blocks * usage.blocksize) as f64 / 1024.0 / 1024.0 / 1024.0;

        Ok(Self {
            free_space_gib: gib_free as u64,
            total_space_gib: gib_total as u64,
            free_space_percent: 100 - percent_used as u64,
        })
    }
}

impl std::fmt::Display for FreeSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}% ({} / {} GiB)",
            self.free_space_percent, self.free_space_gib, self.total_space_gib
        )
    }
}
