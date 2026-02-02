/*!
    Codec identification.
*/

/**
    Codec identifiers.

    This is a subset of codecs commonly used in media pipelines.
    Not all FFmpeg codecs are represented.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodecId {
    // Video codecs
    /// H.264 / AVC
    H264,
    /// H.265 / HEVC
    H265,
    /// VP8
    Vp8,
    /// VP9
    Vp9,
    /// AV1
    Av1,
    /// MPEG-4 Part 2
    Mpeg4,
    /// MPEG-2 Video
    Mpeg2Video,

    // Audio codecs
    /// AAC (Advanced Audio Coding)
    Aac,
    /// Opus
    Opus,
    /// MP3 (MPEG Audio Layer 3)
    Mp3,
    /// Vorbis
    Vorbis,
    /// FLAC (Free Lossless Audio Codec)
    Flac,
    /// PCM signed 16-bit little-endian
    PcmS16Le,
    /// PCM signed 16-bit big-endian
    PcmS16Be,
    /// PCM 32-bit float little-endian
    PcmF32Le,
    /// AC-3 (Dolby Digital)
    Ac3,
}

impl CodecId {
    /**
        Returns true if this is a video codec.
    */
    pub const fn is_video(self) -> bool {
        matches!(
            self,
            Self::H264
                | Self::H265
                | Self::Vp8
                | Self::Vp9
                | Self::Av1
                | Self::Mpeg4
                | Self::Mpeg2Video
        )
    }

    /**
        Returns true if this is an audio codec.
    */
    pub const fn is_audio(self) -> bool {
        matches!(
            self,
            Self::Aac
                | Self::Opus
                | Self::Mp3
                | Self::Vorbis
                | Self::Flac
                | Self::PcmS16Le
                | Self::PcmS16Be
                | Self::PcmF32Le
                | Self::Ac3
        )
    }

    /**
        Returns true if this is a lossless codec.
    */
    pub const fn is_lossless(self) -> bool {
        matches!(
            self,
            Self::Flac | Self::PcmS16Le | Self::PcmS16Be | Self::PcmF32Le
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_codecs() {
        assert!(CodecId::H264.is_video());
        assert!(CodecId::H265.is_video());
        assert!(CodecId::Vp9.is_video());
        assert!(CodecId::Av1.is_video());
        assert!(!CodecId::Aac.is_video());
    }

    #[test]
    fn audio_codecs() {
        assert!(CodecId::Aac.is_audio());
        assert!(CodecId::Opus.is_audio());
        assert!(CodecId::Mp3.is_audio());
        assert!(CodecId::Flac.is_audio());
        assert!(!CodecId::H264.is_audio());
    }

    #[test]
    fn lossless_codecs() {
        assert!(CodecId::Flac.is_lossless());
        assert!(CodecId::PcmS16Le.is_lossless());
        assert!(!CodecId::Aac.is_lossless());
        assert!(!CodecId::Mp3.is_lossless());
    }

    #[test]
    fn codec_is_copy() {
        let c = CodecId::H264;
        let c2 = c;
        assert_eq!(c, c2);
    }
}
