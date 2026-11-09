//! Short "used lately" lists: the last Size limits, offered next to the Size
//! limit field, and the last Tools, shown on Home. Both are kept whether or
//! not History is on.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::store::{JsonFile, Result};

/// How many Size limits are offered; Kompres Foto and Kompres PDF share them.
pub const RECENT_LIMITS: usize = 5;
/// How many recently used Tools are kept; Home shows them first in its grid.
pub const RECENT_TOOLS: usize = crate::catalogue::HOME_TOOLS;

/// The last few values used, newest first, each once.
pub struct Recent<T> {
    file: JsonFile<Vec<T>>,
    cap: usize,
}

/// Size limits in bytes.
pub type RecentLimits = Recent<u64>;
/// Tools by `ToolId::as_str`; ids from before 0.4.1 may remain, so read them
/// with `ToolId::parse`.
pub type RecentTools = Recent<String>;

impl RecentLimits {
    pub fn limits(path: PathBuf) -> Self {
        Self::new(path, RECENT_LIMITS)
    }
}

impl RecentTools {
    pub fn tools(path: PathBuf) -> Self {
        Self::new(path, RECENT_TOOLS)
    }
}

impl<T: Serialize + DeserializeOwned + PartialEq> Recent<T> {
    fn new(path: PathBuf, cap: usize) -> Self {
        Self {
            file: JsonFile::new(path),
            cap,
        }
    }

    /// Newest first.
    pub fn list(&self) -> Result<Vec<T>> {
        self.file.read()
    }

    /// Puts `value` first, dropping an older copy and anything beyond the cap.
    pub fn record(&self, value: T) -> Result<()> {
        let cap = self.cap;
        self.file.update(|list| {
            list.retain(|old| *old != value);
            list.insert(0, value);
            list.truncate(cap);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(test: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("rantakit-recent-tests")
            .join(test);
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("recent.json")
    }

    #[test]
    fn the_last_five_limits_newest_first_without_duplicates() {
        let limits = RecentLimits::limits(scratch("limits"));
        assert!(limits.list().unwrap().is_empty());

        for kb in [100, 200, 300, 200, 400, 500, 600] {
            limits.record(kb * 1024).unwrap();
        }
        let kb: Vec<u64> = limits.list().unwrap().iter().map(|b| b / 1024).collect();
        assert_eq!(kb, [600, 500, 400, 200, 300]);
    }

    #[test]
    fn the_last_seven_tools_newest_first_without_duplicates() {
        let tools = RecentTools::tools(scratch("tools"));
        let used = ["kompres-foto", "kompres-pdf", "kompres-foto", "a", "b", "c", "d", "e", "f"];
        for tool in used {
            tools.record(tool.into()).unwrap();
        }
        assert_eq!(
            tools.list().unwrap(),
            ["f", "e", "d", "c", "b", "a", "kompres-foto"]
        );
    }
}
