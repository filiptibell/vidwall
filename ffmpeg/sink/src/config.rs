/*!
    Sink configuration types.
*/

use std::time::Duration;

use ffmpeg_types::{AudioStreamInfo, VideoStreamInfo};

/**
    Container format for output.
*/
#[derive(Clone, Debug)]
pub enum ContainerFormat {
    /// MP4 container (most compatible).
    Mp4,
    /// Matroska container (most flexible).
    Mkv,
    /// MPEG transport stream.
    MpegTs,
    /// HLS output (generates playlist and segments).
    Hls {
        /// Duration of each segment.
        segment_duration: Duration,
    },
}

impl ContainerFormat {
    /**
        Get the FFmpeg format name for this container.
    */
    pub fn ffmpeg_format_name(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "matroska",
            Self::MpegTs => "mpegts",
            Self::Hls { .. } => "hls",
        }
    }

    /**
        Get the typical file extension for this container.
    */
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::MpegTs => "ts",
            Self::Hls { .. } => "m3u8",
        }
    }
}

/**
    Configuration for a media sink.
*/
#[derive(Clone, Debug)]
pub struct SinkConfig {
    /// Container format to use.
    pub format: ContainerFormat,
    /// Video stream info (None if no video).
    pub video: Option<VideoStreamInfo>,
    /// Audio stream info (None if no audio).
    pub audio: Option<AudioStreamInfo>,
    /// Enable "fast start" for MP4 (moves moov atom to beginning).
    pub fast_start: bool,
}

impl SinkConfig {
    /**
        Create a new sink configuration.
    */
    pub fn new(format: ContainerFormat) -> Self {
        Self {
            format,
            video: None,
            audio: None,
            fast_start: true,
        }
    }

    /**
        Create configuration for MP4 output.
    */
    pub fn mp4() -> Self {
        Self::new(ContainerFormat::Mp4)
    }

    /**
        Create configuration for MKV output.
    */
    pub fn mkv() -> Self {
        Self::new(ContainerFormat::Mkv)
    }

    /**
        Create configuration for MPEG-TS output.
    */
    pub fn mpegts() -> Self {
        Self::new(ContainerFormat::MpegTs)
    }

    /**
        Create configuration for HLS output.
    */
    pub fn hls(segment_duration: Duration) -> Self {
        Self::new(ContainerFormat::Hls { segment_duration })
    }

    /**
        Set video stream info.
    */
    pub fn with_video(mut self, info: VideoStreamInfo) -> Self {
        self.video = Some(info);
        self
    }

    /**
        Set audio stream info.
    */
    pub fn with_audio(mut self, info: AudioStreamInfo) -> Self {
        self.audio = Some(info);
        self
    }

    /**
        Enable or disable fast start for MP4.
    */
    pub fn with_fast_start(mut self, enabled: bool) -> Self {
        self.fast_start = enabled;
        self
    }
}
