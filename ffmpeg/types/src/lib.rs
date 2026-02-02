/*!
    Shared types for the ffmpeg crate ecosystem.

    This crate defines the vocabulary of the ecosystem — the types that cross crate
    boundaries. It has no dependency on FFmpeg, making it lightweight and enabling
    consumers to depend on it without pulling in FFmpeg bindings.

    # Core Types

    - [`Rational`] - Rational numbers for time bases and frame rates
    - [`Pts`] and [`MediaDuration`] - Timestamps in time_base units
    - [`VideoFrame`] and [`AudioFrame`] - Decoded frame data
    - [`Packet`] - Encoded packet data

    # Format Types

    - [`PixelFormat`] - Video pixel formats
    - [`SampleFormat`] - Audio sample formats
    - [`ChannelLayout`] - Audio channel layouts
    - [`CodecId`] - Codec identifiers

    # Stream Information

    - [`VideoStreamInfo`] and [`AudioStreamInfo`] - Stream metadata
    - [`MediaInfo`] - Combined media information

    # Clock and Synchronization

    - [`Clock`] - Trait for playback clocks
    - [`AudioClock`] - Audio-driven clock for A/V sync
    - [`WallClock`] - Wall-time clock for videos without audio

    # Error Handling

    - [`Error`] and [`Result`] - Common error types

    # Pipeline Control

    - [`PipelineSignal`] - Signals for flush and end-of-stream
    - [`StreamType`] - Video or audio stream type
*/

mod clock;
mod codec;
mod error;
mod format;
mod frame;
mod packet;
mod rational;
mod signal;
mod stream;
mod timestamp;

pub use clock::{AudioClock, Clock, WallClock};
pub use codec::CodecId;
pub use error::{Error, Result};
pub use format::{ChannelLayout, PixelFormat, SampleFormat};
pub use frame::{AudioFrame, VideoFrame};
pub use packet::{Packet, StreamType};
pub use rational::Rational;
pub use signal::PipelineSignal;
pub use stream::{AudioStreamInfo, MediaInfo, VideoStreamInfo};
pub use timestamp::{MediaDuration, Pts};
