/*!
    Decoded frame types.
*/

use crate::{ChannelLayout, PixelFormat, Pts, Rational, SampleFormat};

/**
    A decoded video frame.

    Contains raw pixel data in the format specified by `format`.
    The data layout depends on the pixel format — packed formats have
    all data in a single contiguous buffer, while planar formats may
    require interpretation based on the format.
*/
#[derive(Clone, Debug)]
pub struct VideoFrame {
    /// Raw pixel data.
    pub data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Pixel format of the data.
    pub format: PixelFormat,
    /// Presentation timestamp (None for frames without timing).
    pub pts: Option<Pts>,
    /// Time base for interpreting the PTS.
    pub time_base: Rational,
}

impl VideoFrame {
    /**
        Create a new video frame.
    */
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: PixelFormat,
        pts: Option<Pts>,
        time_base: Rational,
    ) -> Self {
        Self {
            data,
            width,
            height,
            format,
            pts,
            time_base,
        }
    }

    /**
        Returns the presentation time as a Duration, if PTS is set.
    */
    pub fn presentation_time(&self) -> Option<std::time::Duration> {
        self.pts.map(|pts| pts.to_duration(self.time_base))
    }
}

/**
    A decoded audio frame.

    Contains raw sample data in the format specified by `format`.
    Samples are interleaved for multi-channel audio.
*/
#[derive(Clone, Debug)]
pub struct AudioFrame {
    /**
        Raw sample data as bytes.

        Interpret according to `format` and `channels`.
        For interleaved stereo F32: [L0, R0, L1, R1, ...]
    */
    pub data: Vec<u8>,
    /**
        Number of samples per channel.
    */
    pub samples: usize,
    /**
        Sample rate in Hz.
    */
    pub sample_rate: u32,
    /**
        Channel layout.
    */
    pub channels: ChannelLayout,
    /**
        Sample format.
    */
    pub format: SampleFormat,
    /**
        Presentation timestamp (None for frames without timing).
    */
    pub pts: Option<Pts>,
    /**
        Time base for interpreting the PTS.
    */
    pub time_base: Rational,
}

impl AudioFrame {
    /**
        Create a new audio frame.
    */
    pub fn new(
        data: Vec<u8>,
        samples: usize,
        sample_rate: u32,
        channels: ChannelLayout,
        format: SampleFormat,
        pts: Option<Pts>,
        time_base: Rational,
    ) -> Self {
        Self {
            data,
            samples,
            sample_rate,
            channels,
            format,
            pts,
            time_base,
        }
    }

    /**
        Returns the presentation time as a Duration, if PTS is set.
    */
    pub fn presentation_time(&self) -> Option<std::time::Duration> {
        self.pts.map(|pts| pts.to_duration(self.time_base))
    }

    /**
        Returns the duration of this frame based on sample count and rate.
    */
    pub fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.samples as f64 / self.sample_rate as f64)
    }

    /**
        Returns the total number of samples (samples per channel * channels).
    */
    pub fn total_samples(&self) -> usize {
        self.samples * self.channels.channels() as usize
    }

    /**
        Returns the expected data length in bytes.
    */
    pub fn expected_data_len(&self) -> usize {
        self.total_samples() * self.format.bytes_per_sample()
    }
}

// Ensure frames are Send + Sync
static_assertions::assert_impl_all!(VideoFrame: Send, Sync);
static_assertions::assert_impl_all!(AudioFrame: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const TB_1_1000: Rational = Rational { num: 1, den: 1000 };

    #[test]
    fn video_frame_construction() {
        let frame = VideoFrame::new(
            vec![0u8; 100 * 100 * 4],
            100,
            100,
            PixelFormat::Bgra,
            Some(Pts(1000)),
            TB_1_1000,
        );

        assert_eq!(frame.width, 100);
        assert_eq!(frame.height, 100);
        assert_eq!(frame.format, PixelFormat::Bgra);
        assert_eq!(frame.data.len(), 100 * 100 * 4);
    }

    #[test]
    fn video_frame_presentation_time() {
        let frame = VideoFrame::new(
            vec![],
            100,
            100,
            PixelFormat::Bgra,
            Some(Pts(1500)),
            TB_1_1000,
        );

        assert_eq!(frame.presentation_time(), Some(Duration::from_millis(1500)));
    }

    #[test]
    fn video_frame_no_pts() {
        let frame = VideoFrame::new(vec![], 100, 100, PixelFormat::Bgra, None, TB_1_1000);

        assert_eq!(frame.presentation_time(), None);
    }

    #[test]
    fn audio_frame_construction() {
        let frame = AudioFrame::new(
            vec![0u8; 1024 * 2 * 4], // 1024 samples, stereo, F32
            1024,
            48000,
            ChannelLayout::Stereo,
            SampleFormat::F32,
            Some(Pts(0)),
            TB_1_1000,
        );

        assert_eq!(frame.samples, 1024);
        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.channels, ChannelLayout::Stereo);
        assert_eq!(frame.format, SampleFormat::F32);
    }

    #[test]
    fn audio_frame_duration() {
        let frame = AudioFrame::new(
            vec![],
            48000, // 1 second worth at 48kHz
            48000,
            ChannelLayout::Stereo,
            SampleFormat::F32,
            None,
            TB_1_1000,
        );

        assert_eq!(frame.duration(), Duration::from_secs(1));
    }

    #[test]
    fn audio_frame_total_samples() {
        let frame = AudioFrame::new(
            vec![],
            1024,
            48000,
            ChannelLayout::Stereo,
            SampleFormat::F32,
            None,
            TB_1_1000,
        );

        assert_eq!(frame.total_samples(), 1024 * 2); // stereo
    }

    #[test]
    fn audio_frame_expected_data_len() {
        let frame = AudioFrame::new(
            vec![],
            1024,
            48000,
            ChannelLayout::Stereo,
            SampleFormat::F32,
            None,
            TB_1_1000,
        );

        // 1024 samples * 2 channels * 4 bytes per F32
        assert_eq!(frame.expected_data_len(), 1024 * 2 * 4);
    }
}
