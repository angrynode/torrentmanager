use chrono::{Duration, TimeZone, Utc};
use hightorrent_api::hightorrent::{SingleTarget, Torrent, TorrentList};
use serde::Deserialize;

use crate::state::{AppState, error::AppStateError};

#[derive(Clone, Debug)]
pub struct TorrentListView {
    // TODO: errors and warnings
    pub filtered_list: Vec<Torrent>,
    pub counter: TorrentListCounter,
}

impl TorrentListView {
    pub async fn apply_request(
        req: TorrentListViewRequest,
        state: &AppState,
    ) -> Result<Self, AppStateError> {
        // Fetch torrent list from torrent backend
        let list = state.torrent_list().await?;

        // Filter data
        let counter = TorrentListCounter::from_torrent_list(&list);
        let filtered_list = req.filter_torrent_list(list);

        Ok(Self {
            counter,
            filtered_list,
        })
    }
}

/// Requested torrent view according to the URL route
///
/// Details:
///
/// - everything/ongoing/stuck/unmanaged filters the global list accordingly
/// - HASH/TORRENTID only selects a specific torrent
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum TorrentListViewRequest {
    ListFilter(TorrentListFilter),
    SingleTorrent(SingleTarget),
}

impl TorrentListViewRequest {
    pub fn filter_torrent_list(&self, list: TorrentList) -> Vec<Torrent> {
        match self {
            Self::ListFilter(filter) => filter.filter_torrent_list(list),
            // TODO: errors
            Self::SingleTorrent(target) => vec![list.get(target).unwrap()],
        }
    }

    pub fn is_filter(&self, torrent_list_filter: TorrentListFilter) -> bool {
        match self {
            Self::ListFilter(filter) => filter == &torrent_list_filter,
            _ => false,
        }
    }
}

/// A specific filter applied to the torrent list.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TorrentListFilter {
    /// All torrents.
    Everything,
    Ongoing,
    Stuck,
    Unmanaged,
    Ok,
}

impl TorrentListFilter {
    pub fn filter_torrent_list(&self, list: TorrentList) -> Vec<Torrent> {
        let mut list: Vec<Torrent> = match self {
            Self::Everything => list.into_iter().collect(),
            Self::Ongoing => list.into_iter().filter(is_torrent_ongoing).collect(),
            Self::Ok => list.into_iter().filter(is_torrent_ok).collect(),
            Self::Stuck => list.into_iter().filter(is_torrent_stuck).collect(),
            Self::Unmanaged => list.into_iter().filter(is_torrent_unmanaged).collect(),
        };

        // Sort list by the latest added torrent (reverse order date_start)
        list.sort_unstable_by_key(|b| std::cmp::Reverse(b.date_start));
        list
    }
}

/// Number of torrents in each state
#[derive(Clone, Debug, Default)]
pub struct TorrentListCounter {
    /// All torrents combined.
    pub everything: u32,
    /// Incomplete torrents actively downloading.
    pub ongoing: u32,
    /// Incomplete torrents downloading for over 24h.
    pub stuck: u32,
    /// Torrents not known to TorrentManager.
    // TODO: this is actually not implemented yet
    pub unmanaged: u32,
    /// Torrent successfully downloaded
    pub ok: u32,
}

impl TorrentListCounter {
    pub fn from_torrent_list(list: &TorrentList) -> Self {
        let mut counter = Self::default();

        for torrent in list {
            counter.everything += 1;

            if is_torrent_unmanaged(torrent) {
                counter.unmanaged += 1;
                continue;
            }

            if is_torrent_ongoing(torrent) {
                counter.ongoing += 1;
                continue;
            }

            if is_torrent_stuck(torrent) {
                counter.stuck += 1;
                continue;
            }

            if is_torrent_ok(torrent) {
                counter.ok += 1;
                continue;
            }

            // Otherwise, the torrent is simply seeding,
            // and we simply don't care.
        }

        counter
    }
}

/// Check if torrent is known to TorrentManager.
// TODO: not implemented yet
pub fn is_torrent_unmanaged(_t: &Torrent) -> bool {
    false
}

/// Check if torrent hasn't finished (progress < 100%)
/// and is less than 24h hours.
pub fn is_torrent_ok(t: &Torrent) -> bool {
    t.progress == 100
}

/// Check if torrent hasn't finished (progress < 100%)
/// and is less than 24h hours.
pub fn is_torrent_ongoing(t: &Torrent) -> bool {
    t.progress < 100 && !more_than_x_hours(t.date_start, 24)
}

/// Check if torrent hasn't finished (progress < 100%)
/// and is more than 24h hours.
pub fn is_torrent_stuck(t: &Torrent) -> bool {
    t.progress < 100 && more_than_x_hours(t.date_start, 24)
}

/// Check if more than X hours have elapsed since reference time.
// TODO: maybe move in some utils module if we ever have more time/date stuff.
pub fn more_than_x_hours(ref_time: i64, hours: u64) -> bool {
    let reference_time = Utc.timestamp_opt(ref_time, 0).unwrap();
    let duration = Duration::hours(hours as i64);
    let now = Utc::now();

    reference_time + duration < now
}
