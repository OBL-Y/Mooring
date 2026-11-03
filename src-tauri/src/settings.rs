use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Put windows back automatically when the display setup changes.
    pub auto_restore: bool,
    /// Also restore when displays are removed (undocking), not only when added.
    pub restore_on_undock: bool,
    /// Save and restore desktop icon positions (Windows only).
    pub restore_desktop_icons: bool,
    /// Seconds to wait for displays to settle before restoring.
    pub settle_delay_secs: u64,
    /// Seconds between automatic snapshots.
    pub snapshot_interval_secs: u64,
    /// Look for a new version when Mooring starts and once a day after that.
    pub check_for_updates: bool,
    /// After a restore, put windows of apps opened later in their saved spot.
    pub place_late_windows: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_restore: true,
            restore_on_undock: true,
            restore_desktop_icons: true,
            settle_delay_secs: 3,
            snapshot_interval_secs: 5,
            check_for_updates: true,
            place_late_windows: true,
        }
    }
}

impl Settings {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    /// Loads settings. The flag is true when no settings file existed (first run).
    pub fn load(dir: &Path) -> (Self, bool) {
        match std::fs::read_to_string(Self::path(dir)) {
            Ok(text) => (serde_json::from_str(&text).unwrap_or_default(), false),
            Err(_) => (Self::default(), true),
        }
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(dir), text).map_err(|e| e.to_string())
    }

    pub fn clamp(mut self) -> Self {
        self.settle_delay_secs = self.settle_delay_secs.clamp(1, 30);
        self.snapshot_interval_secs = self.snapshot_interval_secs.clamp(2, 300);
        self
    }
}
