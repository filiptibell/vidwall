/*!
    Pixel and sample format types.
*/

/**
    Video pixel formats.

    This is a subset of formats commonly encountered in media pipelines.
    Not all FFmpeg pixel formats are represented.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PixelFormat {
    /// Planar YUV 4:2:0, 12bpp (most common video format)
    Yuv420p,
    /// Semi-planar YUV 4:2:0, 12bpp (common hardware decoder output)
    Nv12,
    /// Packed BGRA, 32bpp (common for display on macOS/Windows)
    Bgra,
    /// Packed RGBA, 32bpp (common for display)
    Rgba,
    /// Packed RGB, 24bpp
    Rgb24,
    /// Packed BGR, 24bpp
    Bgr24,
    /// Planar YUV 4:2:2, 16bpp
    Yuv422p,
    /// Planar YUV 4:4:4, 24bpp
    Yuv444p,
    /// Planar YUV 4:2:0, 10-bit (HDR content)
    Yuv420p10,
}

impl PixelFormat {
    /**
        Returns the number of bits per pixel for this format.

        For planar formats, this is the average bits per pixel.
    */
    pub const fn bits_per_pixel(self) -> u32 {
        match self {
            Self::Yuv420p | Self::Nv12 => 12,
            Self::Yuv420p10 => 15, // 10 bits * 1.5 planes average
            Self::Yuv422p => 16,
            Self::Rgb24 | Self::Bgr24 | Self::Yuv444p => 24,
            Self::Bgra | Self::Rgba => 32,
        }
    }

    /**
        Returns true if this is a planar format.
    */
    pub const fn is_planar(self) -> bool {
        match self {
            Self::Yuv420p | Self::Yuv422p | Self::Yuv444p | Self::Yuv420p10 => true,
            Self::Nv12 => true, // semi-planar counts as planar
            Self::Bgra | Self::Rgba | Self::Rgb24 | Self::Bgr24 => false,
        }
    }
}

/**
    Audio sample formats.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SampleFormat {
    /// 32-bit floating point, range [-1.0, 1.0]
    F32,
    /// 64-bit floating point
    F64,
    /// Signed 16-bit integer
    S16,
    /// Signed 32-bit integer
    S32,
    /// Unsigned 8-bit integer
    U8,
}

impl SampleFormat {
    /**
        Returns the number of bytes per sample.
    */
    pub const fn bytes_per_sample(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::S16 => 2,
            Self::S32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }

    /**
        Returns true if this is a floating-point format.
    */
    pub const fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
}

/**
    Audio channel layout.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChannelLayout {
    /// Single channel
    Mono,
    /// Left and right channels
    Stereo,
}

impl ChannelLayout {
    /**
        Returns the number of channels.
    */
    pub const fn channels(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_format_bits_per_pixel() {
        assert_eq!(PixelFormat::Yuv420p.bits_per_pixel(), 12);
        assert_eq!(PixelFormat::Bgra.bits_per_pixel(), 32);
        assert_eq!(PixelFormat::Rgb24.bits_per_pixel(), 24);
    }

    #[test]
    fn pixel_format_is_planar() {
        assert!(PixelFormat::Yuv420p.is_planar());
        assert!(PixelFormat::Nv12.is_planar());
        assert!(!PixelFormat::Bgra.is_planar());
        assert!(!PixelFormat::Rgb24.is_planar());
    }

    #[test]
    fn sample_format_bytes_per_sample() {
        assert_eq!(SampleFormat::U8.bytes_per_sample(), 1);
        assert_eq!(SampleFormat::S16.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::F32.bytes_per_sample(), 4);
        assert_eq!(SampleFormat::F64.bytes_per_sample(), 8);
    }

    #[test]
    fn sample_format_is_float() {
        assert!(SampleFormat::F32.is_float());
        assert!(SampleFormat::F64.is_float());
        assert!(!SampleFormat::S16.is_float());
        assert!(!SampleFormat::S32.is_float());
    }

    #[test]
    fn channel_layout_channels() {
        assert_eq!(ChannelLayout::Mono.channels(), 1);
        assert_eq!(ChannelLayout::Stereo.channels(), 2);
    }
}
