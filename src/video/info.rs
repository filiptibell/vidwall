use std::path::PathBuf;
use std::time::Duration;

/// Metadata about a video file, obtained via ffprobe.
#[derive(Debug, Clone)]
pub struct VideoInfo {
    /// Path to the video file
    pub path: PathBuf,
    /// Video width in pixels
    pub width: u32,
    /// Video height in pixels
    pub height: u32,
    /// Video duration (if available)
    pub duration: Option<Duration>,
}

impl VideoInfo {
    /// Create a new VideoInfo instance.
    pub fn new(path: PathBuf, width: u32, height: u32, duration: Option<Duration>) -> Self {
        Self {
            path,
            width,
            height,
            duration,
        }
    }

    /// Calculate the aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}
