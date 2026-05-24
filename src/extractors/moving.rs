use serde::Deserialize;
use serde_with::serde_as;

/// A request to move a torrent around in the categories/folders.
///
/// When validate is set, the requested folder is set to the database.
#[derive(Clone, Debug, Deserialize)]
#[serde_as]
pub struct MovingQuery {
    #[serde(default)]
    #[serde_as(as = "DeserializeFromStr")]
    pub id: Option<i32>,
    #[serde(default)]
    pub validate: bool,
}
