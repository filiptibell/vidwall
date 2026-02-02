/*!
    Media output and muxing for the ffmpeg crate ecosystem.

    This crate handles the output side of the media pipeline. It takes encoded
    packets from the encoder and writes them into container formats — MP4 files,
    MKV files, HLS segments, etc.

    # Basic Usage

    ```ignore
    use ffmpeg_sink::{Sink, SinkConfig, ContainerFormat};

    // Get stream info from encoders
    let video_info = video_encoder.stream_info();
    let audio_info = audio_encoder.stream_info();

    // Create MP4 sink
    let config = SinkConfig::mp4()
        .with_video(video_info)
        .with_audio(audio_info);

    let mut sink = Sink::file("output.mp4", config)?;

    // Write encoded packets
    for packet in encoded_packets {
        sink.write(&packet)?;
    }

    // Finalize the file (critical!)
    sink.finish()?;
    ```

    # Container Formats

    - **MP4**: Most compatible, supports H.264/H.265/AAC
    - **MKV**: Most flexible, supports virtually any codec
    - **MPEG-TS**: Transport stream, good for streaming
    - **HLS**: HTTP Live Streaming (playlist + segments)

    ```ignore
    // MP4 with fast start (moov at beginning for streaming)
    SinkConfig::mp4().with_fast_start(true)

    // MKV
    SinkConfig::mkv()

    // HLS with 4-second segments
    SinkConfig::hls(Duration::from_secs(4))
    ```

    # Stream Configuration

    The sink needs to know about streams before writing. Get this info
    from your encoders:

    ```ignore
    let config = SinkConfig::mp4()
        .with_video(video_encoder.stream_info())
        .with_audio(audio_encoder.stream_info());
    ```

    # Finalization

    Always call `finish()` to properly finalize the container:

    ```ignore
    sink.finish()?;
    ```

    Without this:
    - Duration may be unknown to players
    - Seeking may not work
    - Some players won't open the file

    # Features

    - `file`: File output support (enabled by default)
    - `hls`: HLS output support
*/

pub use ffmpeg_types::{
    AudioStreamInfo, ChannelLayout, CodecId, Error, Packet, PixelFormat, Rational, Result,
    SampleFormat, StreamType, VideoStreamInfo,
};

mod config;
mod sink;

pub use config::{ContainerFormat, SinkConfig};
pub use sink::Sink;
