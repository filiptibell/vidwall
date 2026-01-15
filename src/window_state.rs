use std::fs;
use std::path::PathBuf;

use gpui::{Size, px};
use serde::{Deserialize, Serialize};

/// Saved window state for persistence across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
}

impl WindowState {
    /// Create a new WindowState from GPUI size.
    pub fn from_size(size: Size<gpui::Pixels>) -> Self {
        Self {
            width: size.width.into(),
            height: size.height.into(),
        }
    }

    /// Convert to GPUI size.
    pub fn to_size(&self) -> Size<gpui::Pixels> {
        Size {
            width: px(self.width),
            height: px(self.height),
        }
    }

    /// Get the path to the window state file.
    fn state_file_path() -> Option<PathBuf> {
        dirs::data_local_dir().map(|p| p.join("vidwall").join("window_state.json"))
    }

    /// Load window state from disk.
    pub fn load() -> Option<Self> {
        let path = Self::state_file_path()?;
        let contents = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Save window state to disk.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = match Self::state_file_path() {
            Some(p) => p,
            None => return Ok(()), // Silently skip if no data dir
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)
    }
}
