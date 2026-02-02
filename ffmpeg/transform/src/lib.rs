/*!
    Media frame transformation for the ffmpeg crate ecosystem.

    This crate converts frames between formats:
    - **Video**: scaling, pixel format conversion (YUV → RGB, etc.)
    - **Audio**: resampling, channel layout conversion, sample format conversion

    This is the "adapter" layer that makes decoded frames usable by downstream
    consumers. Decoders output frames in whatever format the codec specifies;
    this crate converts them to the format consumers need.

    # Video Transformation

    ```ignore
    use ffmpeg_transform::{VideoTransform, VideoTransformConfig};
    use ffmpeg_types::PixelFormat;

    // Convert any video frame to 1920x1080 BGRA for display
    let config = VideoTransformConfig::to_bgra(1920, 1080);
    let mut transform = VideoTransform::new(config);

    // Transform frames (scaler lazily initialized on first call)
    for frame in decoded_frames {
        let bgra_frame = transform.transform(&frame)?;
        // Display bgra_frame
    }
    ```

    # Audio Transformation

    ```ignore
    use ffmpeg_transform::{AudioTransform, AudioTransformConfig};

    // Convert any audio to 48kHz stereo F32 for playback
    let config = AudioTransformConfig::playback();
    let mut transform = AudioTransform::new(config);

    // Transform frames
    for frame in decoded_frames {
        let playback_frame = transform.transform(&frame)?;
        // Send to audio output
    }

    // Flush any remaining samples at end of stream
    if let Some(final_frame) = transform.flush()? {
        // Send final samples
    }
    ```

    # Lazy Initialization

    Both transformers lazily initialize their FFmpeg contexts on first use.
    This allows creating transformers before knowing the exact input format.
    If the input format changes mid-stream (rare but possible), the context
    is automatically reinitialized.

    # Stateless vs Stateful

    **Video transformation is stateless**: each frame transforms independently.
    Frames can be processed in any order.

    **Audio transformation is stateful**: the resampler maintains filter history.
    Frames should be processed in order, and `flush()` must be called at end
    of stream to retrieve buffered samples. Call `reset()` after seeking.
*/

pub use ffmpeg_types::{
    AudioFrame, ChannelLayout, Error, PixelFormat, Result, SampleFormat, VideoFrame,
};

mod audio;
mod video;

pub use audio::{AudioTransform, AudioTransformConfig};
pub use video::{ScalingAlgorithm, VideoTransform, VideoTransformConfig};
