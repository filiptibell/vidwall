use std::path::PathBuf;
use std::sync::RwLock;

use rand::seq::SliceRandom;

use super::VideoInfo;

/// Thread-safe storage for validated video files.
///
/// Videos are added to this storage after being validated by ffprobe.
/// The GridView pulls videos from here when filling slots.
pub struct ReadyVideos {
    videos: RwLock<Vec<VideoInfo>>,
}

impl ReadyVideos {
    /// Create a new empty ReadyVideos storage.
    pub fn new() -> Self {
        Self {
            videos: RwLock::new(Vec::new()),
        }
    }

    /// Add a validated video to the storage.
    pub fn push(&self, info: VideoInfo) {
        let mut videos = self.videos.write().unwrap();
        videos.push(info);
    }

    /// Get the number of ready videos.
    pub fn len(&self) -> usize {
        self.videos.read().unwrap().len()
    }

    /// Check if the storage is empty.
    pub fn is_empty(&self) -> bool {
        self.videos.read().unwrap().is_empty()
    }

    /// Pick a random video from the storage.
    ///
    /// Returns None if the storage is empty.
    pub fn pick_random(&self) -> Option<VideoInfo> {
        let videos = self.videos.read().unwrap();
        let mut rng = rand::thread_rng();
        videos.choose(&mut rng).cloned()
    }

    /// Pick a random video that is not in the exclusion list.
    ///
    /// If all videos are in the exclusion list, falls back to picking any random video.
    /// Returns None if the storage is empty.
    pub fn pick_random_except(&self, exclude: &[PathBuf]) -> Option<VideoInfo> {
        let videos = self.videos.read().unwrap();
        if videos.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();

        // Try to find a video not in the exclusion list
        let available: Vec<_> = videos
            .iter()
            .filter(|v| !exclude.contains(&v.path))
            .collect();

        if available.is_empty() {
            // Fall back to any video if all are excluded
            videos.choose(&mut rng).cloned()
        } else {
            available.choose(&mut rng).cloned().cloned()
        }
    }

    /// Get all video paths currently in storage.
    pub fn all_paths(&self) -> Vec<PathBuf> {
        self.videos
            .read()
            .unwrap()
            .iter()
            .map(|v| v.path.clone())
            .collect()
    }
}

impl Default for ReadyVideos {
    fn default() -> Self {
        Self::new()
    }
}
