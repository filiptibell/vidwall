/*!
    Decoder configuration types.
*/

/**
    Hardware device type for hardware-accelerated decoding.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HwDevice {
    /// VideoToolbox (macOS)
    VideoToolbox,
    /// VAAPI (Linux - AMD, Intel)
    Vaapi,
    /// CUDA/NVDEC (NVIDIA)
    Cuda,
    /// Quick Sync Video (Intel)
    Qsv,
}

/**
    Configuration for video decoder.
*/
#[derive(Clone, Debug, Default)]
pub struct VideoDecoderConfig {
    /// Prefer hardware decoding if available.
    pub prefer_hw: bool,
    /// Specific hardware device to use (None = auto-detect).
    pub hw_device: Option<HwDevice>,
}

impl VideoDecoderConfig {
    /**
        Create a new config with default settings (software decoding).
    */
    pub fn new() -> Self {
        Self::default()
    }

    /**
        Create a config that prefers hardware acceleration.
    */
    pub fn with_hw_accel() -> Self {
        Self {
            prefer_hw: true,
            hw_device: None,
        }
    }

    /**
        Create a config with a specific hardware device.
    */
    pub fn with_hw_device(device: HwDevice) -> Self {
        Self {
            prefer_hw: true,
            hw_device: Some(device),
        }
    }
}

/**
    Configuration for audio decoder.

    Audio decoding doesn't typically use hardware acceleration,
    so this config is minimal.
*/
#[derive(Clone, Debug, Default)]
pub struct AudioDecoderConfig {
    // Reserved for future options
}

impl AudioDecoderConfig {
    /**
        Create a new config with default settings.
    */
    pub fn new() -> Self {
        Self::default()
    }
}
