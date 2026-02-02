/*!
    Media decoding for the ffmpeg crate ecosystem.

    This crate transforms encoded packets into raw frames. It handles the
    computationally intensive work of codec decoding, including hardware
    acceleration when available.

    # Features

    - `videotoolbox`: Enable VideoToolbox hardware acceleration (macOS)
    - `vaapi`: Enable VAAPI hardware acceleration (Linux)
    - `cuda`: Enable CUDA/NVDEC hardware acceleration (NVIDIA)

    # Example

    ```ignore
    use ffmpeg_source::{open, StreamType};
    use ffmpeg_decode::{VideoDecoder, VideoDecoderConfig};

    // Open a source
    let mut source = open("video.mp4")?;

    // Create decoder with hardware acceleration
    let config = VideoDecoderConfig::with_hw_accel();
    let codec_config = source.take_video_codec_config().unwrap();
    let time_base = source.video_time_base().unwrap();
    let mut decoder = VideoDecoder::new(codec_config, time_base, config)?;

    // Decode packets
    for packet in source {
        let packet = packet?;
        if packet.is_video() {
            let frames = decoder.decode(&packet)?;
            for frame in frames {
                // Process frame
            }
        }
    }

    // Flush remaining frames
    let remaining = decoder.flush()?;
    ```

    # Hardware Acceleration

    Hardware acceleration is opt-in and falls back to software decoding
    if hardware is unavailable:

    ```ignore
    // Prefer hardware, auto-detect device
    let config = VideoDecoderConfig::with_hw_accel();

    // Specific hardware device
    let config = VideoDecoderConfig::with_hw_device(HwDevice::VideoToolbox);

    // Software only
    let config = VideoDecoderConfig::new();
    ```
*/

pub use ffmpeg_source::CodecConfig;
pub use ffmpeg_types::{AudioFrame, Error, Packet, Result, VideoFrame};

mod audio;
mod config;
mod hw;
mod video;

pub use audio::AudioDecoder;
pub use config::{AudioDecoderConfig, HwDevice, VideoDecoderConfig};
pub use video::VideoDecoder;
