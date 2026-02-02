# FFmpeg Crate Ecosystem Plan

## 1. Overview

### Problem Statement

Building applications that process media (video players, transcoders, streaming servers) requires working with FFmpeg. The `ffmpeg-next` Rust bindings provide low-level access, but every project ends up building similar abstractions: demuxing, decoding, format conversion, encoding, muxing, and crucially — timing and synchronization.

This ecosystem provides reusable, well-tested crates that handle these concerns correctly, so applications can focus on their unique logic rather than re-implementing media pipeline fundamentals.

### Crate Summary

| Crate              | Purpose                                                                            |
| ------------------ | ---------------------------------------------------------------------------------- |
| `ffmpeg-types`     | Shared types with no FFmpeg dependency (frames, packets, timestamps, clock traits) |
| `ffmpeg-source`    | Input abstraction — demuxing from files, HTTP, HLS streams                         |
| `ffmpeg-decode`    | Packet → Frame decoding with hardware acceleration                                 |
| `ffmpeg-transform` | Frame → Frame conversion (scaling, pixel format, resampling)                       |
| `ffmpeg-encode`    | Frame → Packet encoding                                                            |
| `ffmpeg-sink`      | Output abstraction — muxing to files, HLS segments                                 |

### Scope

**In scope:**

- Linear media pipelines (source → decode → transform → encode → sink)
- File and HTTP/HLS input sources
- Hardware-accelerated decoding (VideoToolbox on macOS, others as needed)
- Simple frame transformations (scaling, pixel format conversion, audio resampling)
- Clock and A/V synchronization primitives
- Async support via `reqwest` for streaming inputs

**Non-goals:**

- Complete FFmpeg binding replacement — we target media pipeline use cases only
- Complex filter graphs (libavfilter) — only simple swscale/swresample operations
- Real-time capture (webcams, microphones)
- Subtitle handling
- DRM/content protection

---

## 2. Architecture

### Crate Dependency Graph

```
                    ffmpeg-types
                    (no ffmpeg-next)
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
   ffmpeg-source    ffmpeg-decode    ffmpeg-transform
   (async, reqwest) (hw accel)       (swscale, swresample)
        │                │                │
        │                ▼                │
        │          ffmpeg-encode         │
        │                │                │
        └───────────────►│◄───────────────┘
                         ▼
                    ffmpeg-sink
```

All crates depend on `ffmpeg-types`. The `ffmpeg-source`, `ffmpeg-decode`, `ffmpeg-transform`, `ffmpeg-encode`, and `ffmpeg-sink` crates depend on `ffmpeg-next`.

### Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           PLAYBACK PIPELINE                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────┐     ┌──────────┐     ┌───────────┐     ┌──────────────┐   │
│  │  Source  │────►│  Decode  │────►│ Transform │────►│   Display    │   │
│  │ (demux)  │     │          │     │ (scale)   │     │ (app layer)  │   │
│  └──────────┘     └──────────┘     └───────────┘     └──────────────┘   │
│       │                                                      ▲          │
│       │           ┌──────────┐     ┌───────────┐             │          │
│       └──────────►│  Decode  │────►│ Transform │────►────────┘          │
│      (audio)      │ (audio)  │     │(resample) │    (audio out)         │
│                   └──────────┘     └───────────┘                        │
│                                           │                             │
│                                           ▼                             │
│                                    ┌─────────────┐                      │
│                                    │    Clock    │ ◄── A/V sync         │
│                                    └─────────────┘                      │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                          TRANSCODE PIPELINE                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────┐     ┌──────────┐     ┌───────────┐     ┌──────────┐       │
│  │  Source  │────►│  Decode  │────►│ Transform │────►│  Encode  │       │
│  │          │     │          │     │           │     │          │       │
│  └──────────┘     └──────────┘     └───────────┘     └──────────┘       │
│                                                            │            │
│                                                            ▼            │
│                                                      ┌──────────┐       │
│                                                      │   Sink   │       │
│                                                      │  (mux)   │       │
│                                                      └──────────┘       │
└─────────────────────────────────────────────────────────────────────────┘
```

### Type Flow Between Crates

```
ffmpeg-source  ──► Packet (video)  ──► ffmpeg-decode  ──► VideoFrame
               ──► Packet (audio)  ──► ffmpeg-decode  ──► AudioFrame
                                                              │
                                                              ▼
                                                       ffmpeg-transform
                                                              │
                                                              ▼
                                       VideoFrame (target format) / AudioFrame (target format)
                                                              │
                                                              ▼
                                                       ffmpeg-encode
                                                              │
                                                              ▼
                                                          Packet
                                                              │
                                                              ▼
                                                        ffmpeg-sink
```

---

## 3. Crate Specifications

### 3.1 `ffmpeg-types`

**Purpose:** Shared types that define the interfaces between crates. Has no dependency on `ffmpeg-next`, enabling downstream crates to depend on it without pulling in FFmpeg.

**Dependencies:**

- `std` only (no external dependencies)

**Public API Surface:**

```rust
/// Rational number for time bases and frame rates
pub struct Rational {
    pub num: i32,
    pub den: i32,
}

/// Presentation timestamp in time_base units
pub struct Pts(pub i64);

/// Duration in time_base units
pub struct MediaDuration(pub i64);

/// Convert between Pts/MediaDuration and std::time::Duration
impl Pts {
    pub fn to_duration(&self, time_base: Rational) -> Duration;
}

/// Video pixel formats (subset we support)
pub enum PixelFormat {
    Yuv420p,
    Nv12,
    Bgra,
    Rgba,
    // ... other common formats
}

/// Audio sample formats
pub enum SampleFormat {
    F32,
    S16,
    S32,
}

/// Audio channel layout
pub enum ChannelLayout {
    Mono,
    Stereo,
}

/// Decoded video frame — owns its pixel data
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub pts: Option<Pts>,
    pub time_base: Rational,
}

/// Decoded audio frame — owns its sample data
pub struct AudioFrame {
    pub data: Vec<u8>,  // Raw bytes, interpreted per format
    pub samples: usize,
    pub sample_rate: u32,
    pub channels: ChannelLayout,
    pub format: SampleFormat,
    pub pts: Option<Pts>,
    pub time_base: Rational,
}

/// Encoded packet — codec-agnostic
pub struct Packet {
    pub data: Vec<u8>,
    pub pts: Option<Pts>,
    pub dts: Option<Pts>,
    pub duration: MediaDuration,
    pub time_base: Rational,
    pub is_keyframe: bool,
    pub stream_type: StreamType,
}

pub enum StreamType {
    Video,
    Audio,
}

/// Signals for pipeline control (seeking, flush)
pub enum PipelineSignal {
    /// Flush buffers — discontinuity in stream (e.g., after seek)
    Flush,
    /// End of stream
    Eos,
}

/// Stream metadata
pub struct VideoStreamInfo {
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub frame_rate: Option<Rational>,
    pub time_base: Rational,
    pub duration: Option<Duration>,
    pub codec_id: CodecId,
}

pub struct AudioStreamInfo {
    pub sample_rate: u32,
    pub channels: ChannelLayout,
    pub sample_format: SampleFormat,
    pub time_base: Rational,
    pub duration: Option<Duration>,
    pub codec_id: CodecId,
}

/// Codec identifiers (subset we support)
pub enum CodecId {
    H264,
    H265,
    Vp9,
    Av1,
    Aac,
    Opus,
    Mp3,
    // ...
}

/// Error types
pub enum Error {
    Io(std::io::Error),
    Codec { message: String },
    InvalidData { message: String },
    UnsupportedFormat { message: String },
    Eof,
}
```

**Clock/Sync Types:**

```rust
/// Trait for playback clocks — provides current playback position
pub trait Clock: Send + Sync {
    /// Get current playback position
    fn position(&self) -> Duration;

    /// Reset clock to a specific position (for seeking)
    fn reset_to(&self, position: Duration);
}

/// Audio-driven clock that tracks position based on samples consumed
pub struct AudioClock {
    // ... implementation details not specified
}

impl AudioClock {
    pub fn new(sample_rate: u32, channels: u16) -> Self;

    /// Called by audio consumer when samples are consumed
    pub fn add_samples(&self, count: u64);

    /// Mark audio as finished — clock will extrapolate using wall time
    pub fn mark_finished(&self);
}

impl Clock for AudioClock { /* ... */ }

/// Wall-time clock for videos without audio
pub struct WallClock {
    // ... implementation details not specified
}

impl WallClock {
    pub fn new() -> Self;
    pub fn pause(&self);
    pub fn resume(&self);
}

impl Clock for WallClock { /* ... */ }
```

**Responsibilities:**

- Define all types that cross crate boundaries
- Provide timestamp/duration conversion utilities
- Define clock traits for A/V synchronization
- Provide concrete clock implementations (`AudioClock`, `WallClock`)

**Non-Responsibilities:**

- No FFmpeg interaction
- No I/O operations
- No codec-specific logic

---

### 3.2 `ffmpeg-source`

**Purpose:** Input abstraction — opens media from various sources (files, HTTP, HLS) and produces encoded packets. Handles demuxing and stream selection.

**Dependencies:**

- `ffmpeg-types`
- `ffmpeg-next`
- `reqwest` (for HTTP/HLS)
- `tokio` (async runtime)

**Public API Surface:**

```rust
/// Source configuration
pub struct SourceConfig {
    /// Which streams to demux (None = all)
    pub stream_filter: Option<StreamFilter>,
}

pub enum StreamFilter {
    VideoOnly,
    AudioOnly,
    Both,
}

/// Probe media without fully opening — returns stream info
pub async fn probe(url: &str) -> Result<MediaInfo, Error>;
pub fn probe_sync(path: &Path) -> Result<MediaInfo, Error>;

pub struct MediaInfo {
    pub duration: Option<Duration>,
    pub video: Option<VideoStreamInfo>,
    pub audio: Option<AudioStreamInfo>,
}

/// Open a media source (file path or URL)
pub async fn open(url: &str, config: SourceConfig) -> Result<Source, Error>;
pub fn open_sync(path: &Path, config: SourceConfig) -> Result<Source, Error>;

/// Active media source — produces packets
pub struct Source {
    // ... internal state
}

impl Source {
    /// Get stream information
    pub fn video_info(&self) -> Option<&VideoStreamInfo>;
    pub fn audio_info(&self) -> Option<&AudioStreamInfo>;

    /// Read next packet (async)
    pub async fn next_packet(&mut self) -> Result<Option<Packet>, Error>;

    /// Read next packet (sync)
    pub fn next_packet_sync(&mut self) -> Result<Option<Packet>, Error>;

    /// Seek to position
    pub async fn seek(&mut self, position: Duration) -> Result<(), Error>;
    pub fn seek_sync(&mut self, position: Duration) -> Result<(), Error>;
}

/// Streaming source for async iteration
impl Stream for Source {
    type Item = Result<Packet, Error>;
}
```

**Responsibilities:**

- Open files and URLs (HTTP, HLS, MPEG-TS)
- Parse container formats (demuxing)
- Extract stream metadata
- Produce encoded packets with proper timestamps
- Handle seeking at the container level
- Manage async I/O for network sources

**Non-Responsibilities:**

- Decoding packets (that's `ffmpeg-decode`)
- Any frame-level operations
- Clock/sync (packets just carry timestamps)

**Design Decisions:**

- Separate sync and async APIs rather than forcing async everywhere
- `reqwest` for HTTP because it's well-maintained and supports streaming
- Stream filtering at open time to avoid demuxing unwanted streams

---

### 3.3 `ffmpeg-decode`

**Purpose:** Decode encoded packets into raw frames. Supports hardware acceleration where available.

**Dependencies:**

- `ffmpeg-types`
- `ffmpeg-next`

**Public API Surface:**

```rust
/// Decoder configuration
pub struct DecoderConfig {
    /// Prefer hardware decoding if available
    pub prefer_hw: bool,
    /// Specific HW device to use (None = auto-detect)
    pub hw_device: Option<HwDevice>,
}

pub enum HwDevice {
    VideoToolbox,  // macOS
    Vaapi,         // Linux
    Cuda,          // NVIDIA
    Qsv,           // Intel
}

/// Create a video decoder
pub fn video_decoder(
    stream_info: &VideoStreamInfo,
    config: DecoderConfig,
) -> Result<VideoDecoder, Error>;

/// Create an audio decoder
pub fn audio_decoder(
    stream_info: &AudioStreamInfo,
) -> Result<AudioDecoder, Error>;

pub struct VideoDecoder {
    // ... internal state
}

impl VideoDecoder {
    /// Decode a packet, returning decoded frames
    /// Multiple frames may be returned (B-frames, etc.)
    pub fn decode(&mut self, packet: &Packet) -> Result<Vec<VideoFrame>, Error>;

    /// Flush decoder at end of stream
    pub fn flush(&mut self) -> Result<Vec<VideoFrame>, Error>;

    /// Signal discontinuity (after seek) — clears internal buffers
    pub fn reset(&mut self);

    /// Check if using hardware acceleration
    pub fn is_hw_accelerated(&self) -> bool;
}

pub struct AudioDecoder {
    // ... internal state
}

impl AudioDecoder {
    /// Decode a packet, returning decoded frames
    pub fn decode(&mut self, packet: &Packet) -> Result<Vec<AudioFrame>, Error>;

    /// Flush decoder at end of stream
    pub fn flush(&mut self) -> Result<Vec<AudioFrame>, Error>;

    /// Signal discontinuity (after seek)
    pub fn reset(&mut self);
}
```

**Responsibilities:**

- Create codec contexts from stream parameters
- Decode packets to frames
- Handle hardware acceleration setup and frame transfer (GPU → CPU)
- Handle decoder flush at end of stream
- Handle discontinuities (reset after seek)

**Non-Responsibilities:**

- Demuxing (that's `ffmpeg-source`)
- Frame transformation (that's `ffmpeg-transform`)
- Managing decode threads (consumer's responsibility)

**Design Decisions:**

- Hardware acceleration is opt-in with automatic fallback
- Decoder owns its codec context — not shared
- `decode()` returns `Vec<VideoFrame>` because decoders may buffer frames internally
- Frames come out in whatever pixel format the decoder produces — transformation is separate

---

### 3.4 `ffmpeg-transform`

**Purpose:** Transform frames between formats. Video: scaling, pixel format conversion. Audio: resampling, channel layout conversion.

**Dependencies:**

- `ffmpeg-types`
- `ffmpeg-next`

**Public API Surface:**

```rust
/// Video transform configuration
pub struct VideoTransformConfig {
    pub target_width: u32,
    pub target_height: u32,
    pub target_format: PixelFormat,
    pub scaling_algorithm: ScalingAlgorithm,
}

pub enum ScalingAlgorithm {
    Bilinear,
    Bicubic,
    Lanczos,
    Nearest,
}

/// Audio transform configuration
pub struct AudioTransformConfig {
    pub target_sample_rate: u32,
    pub target_channels: ChannelLayout,
    pub target_format: SampleFormat,
}

/// Create a video transformer
pub fn video_transform(config: VideoTransformConfig) -> VideoTransform;

/// Create an audio transformer
pub fn audio_transform(config: AudioTransformConfig) -> AudioTransform;

pub struct VideoTransform {
    // ... internal state (scaler context, cached based on input format)
}

impl VideoTransform {
    /// Transform a frame to the target format
    /// Lazily initializes/reinitializes scaler if input format changes
    pub fn transform(&mut self, frame: &VideoFrame) -> Result<VideoFrame, Error>;
}

pub struct AudioTransform {
    // ... internal state (resampler context)
}

impl AudioTransform {
    /// Transform audio to target format
    /// Lazily initializes resampler if input format changes
    pub fn transform(&mut self, frame: &AudioFrame) -> Result<AudioFrame, Error>;

    /// Flush any buffered samples at end of stream
    pub fn flush(&mut self) -> Result<Option<AudioFrame>, Error>;
}
```

**Responsibilities:**

- Video scaling (arbitrary dimensions)
- Pixel format conversion (YUV ↔ RGB ↔ BGRA, etc.)
- Audio sample rate conversion
- Audio channel layout conversion (mono ↔ stereo)
- Audio sample format conversion (S16 ↔ F32, etc.)

**Non-Responsibilities:**

- Complex filter graphs
- Effects (blur, color correction, etc.)
- Mixing multiple streams
- Any decode/encode operations

**Design Decisions:**

- Lazy initialization of swscale/swresample contexts
- Automatic re-initialization if input format changes mid-stream
- Simple one-in-one-out model (no multi-input filter graphs)

---

### 3.5 `ffmpeg-encode`

**Purpose:** Encode raw frames into packets for a target codec.

**Dependencies:**

- `ffmpeg-types`
- `ffmpeg-next`

**Public API Surface:**

```rust
/// Video encoder configuration
pub struct VideoEncoderConfig {
    pub codec: CodecId,
    pub width: u32,
    pub height: u32,
    pub frame_rate: Rational,
    pub bitrate: Option<u64>,
    pub preset: Option<EncoderPreset>,
    pub pixel_format: PixelFormat,
}

pub enum EncoderPreset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    Veryslow,
}

/// Audio encoder configuration
pub struct AudioEncoderConfig {
    pub codec: CodecId,
    pub sample_rate: u32,
    pub channels: ChannelLayout,
    pub bitrate: Option<u64>,
    pub sample_format: SampleFormat,
}

/// Create a video encoder
pub fn video_encoder(config: VideoEncoderConfig) -> Result<VideoEncoder, Error>;

/// Create an audio encoder
pub fn audio_encoder(config: AudioEncoderConfig) -> Result<AudioEncoder, Error>;

pub struct VideoEncoder {
    // ... internal state
}

impl VideoEncoder {
    /// Encode a frame, returning encoded packets
    pub fn encode(&mut self, frame: &VideoFrame) -> Result<Vec<Packet>, Error>;

    /// Flush encoder at end of stream
    pub fn flush(&mut self) -> Result<Vec<Packet>, Error>;

    /// Get stream info for muxer
    pub fn stream_info(&self) -> VideoStreamInfo;
}

pub struct AudioEncoder {
    // ... internal state
}

impl AudioEncoder {
    /// Encode a frame, returning encoded packets
    pub fn encode(&mut self, frame: &AudioFrame) -> Result<Vec<Packet>, Error>;

    /// Flush encoder at end of stream
    pub fn flush(&mut self) -> Result<Vec<Packet>, Error>;

    /// Get stream info for muxer
    pub fn stream_info(&self) -> AudioStreamInfo;
}
```

**Responsibilities:**

- Create encoder contexts with appropriate settings
- Encode frames to packets
- Handle encoder flush at end of stream
- Provide stream info for muxer setup

**Non-Responsibilities:**

- Decoding
- Frame transformation
- Muxing (that's `ffmpeg-sink`)

**Design Decisions:**

- `encode()` returns `Vec<Packet>` because encoders buffer frames
- Codec-specific options via preset enum rather than exposing all FFmpeg options
- Input frames must match encoder's expected format — transformation is caller's responsibility

---

### 3.6 `ffmpeg-sink`

**Purpose:** Output abstraction — muxes encoded packets into container formats and writes to files or streams.

**Dependencies:**

- `ffmpeg-types`
- `ffmpeg-next`
- `tokio` (for async file I/O if needed)

**Public API Surface:**

```rust
/// Sink configuration
pub struct SinkConfig {
    pub format: ContainerFormat,
    pub video: Option<VideoStreamInfo>,
    pub audio: Option<AudioStreamInfo>,
}

pub enum ContainerFormat {
    Mp4,
    Mkv,
    Hls { segment_duration: Duration },
    MpegTs,
}

/// Create a file sink
pub fn file_sink(path: &Path, config: SinkConfig) -> Result<Sink, Error>;

/// Create an HLS sink (writes segments and playlist)
pub fn hls_sink(
    output_dir: &Path,
    playlist_name: &str,
    config: SinkConfig,
) -> Result<Sink, Error>;

pub struct Sink {
    // ... internal state
}

impl Sink {
    /// Write a packet to the sink
    pub fn write(&mut self, packet: &Packet) -> Result<(), Error>;

    /// Finalize and close the sink
    pub fn finish(self) -> Result<(), Error>;
}
```

**Responsibilities:**

- Create output contexts
- Mux packets from multiple streams into containers
- Write container headers/trailers
- Handle HLS segment generation

**Non-Responsibilities:**

- Encoding (that's `ffmpeg-encode`)
- Any frame-level operations

**Design Decisions:**

- HLS as first-class output format given the streaming use case
- `finish()` consumes self to ensure proper finalization

---

## 4. Cross-Cutting Concerns

### Error Handling

All crates use a common `Error` type from `ffmpeg-types`:

```rust
pub enum Error {
    Io(std::io::Error),
    Codec { message: String },
    InvalidData { message: String },
    UnsupportedFormat { message: String },
    Eof,
}
```

Each crate re-exports this type and may define additional error variants via extension traits if needed. FFmpeg errors are converted to `Error::Codec` with the FFmpeg error message.

### Async Model

- `ffmpeg-source` provides both sync and async APIs
    - Async uses `tokio` runtime and `reqwest` for HTTP
    - Sync API for simple file-based usage
- `ffmpeg-decode`, `ffmpeg-transform`, `ffmpeg-encode` are sync-only (CPU-bound work)
- `ffmpeg-sink` is sync (file I/O is typically fast enough)

Applications can run decode/transform/encode in blocking tasks (`tokio::task::spawn_blocking`) if needed.

### Threading Model

Crates do **not** spawn threads internally. Thread management is the consumer's responsibility. This allows consumers to:

- Use their own executor (tokio, async-std, custom)
- Control thread pool sizes
- Implement custom pipeline architectures

The `vidwall` pattern (separate demux and decode threads per pipeline) is a valid consumer implementation, not something baked into the crates.

### Thread Safety

- Types in `ffmpeg-types` are `Send + Sync` where appropriate
- `VideoDecoder`, `AudioDecoder`, `VideoEncoder`, `AudioEncoder` are `Send` but not `Sync` (single-threaded usage per instance)
- `Clock` trait requires `Send + Sync` — implementations must be thread-safe
- `Source` is `Send` but not `Sync`

### Zero-Copy vs Owned Data

Frames and packets **own their data** (`Vec<u8>`). This is a deliberate trade-off:

**Pros:**

- Simple lifetime management
- Safe to pass across thread boundaries
- No reference counting overhead for most use cases

**Cons:**

- Copies when moving data between pipeline stages

Future optimization: Add `Bytes`-backed variants for zero-copy scenarios if profiling shows it's needed.

### FFmpeg Type Abstraction

Public APIs **never expose `ffmpeg-next` types**. All FFmpeg types are converted to/from `ffmpeg-types` at crate boundaries. This:

- Keeps `ffmpeg-types` dependency-free
- Allows swapping FFmpeg bindings in the future
- Provides a stable API even if `ffmpeg-next` changes

### Unsafe Code Policy

- `ffmpeg-types`: No unsafe code
- Other crates: Unsafe only where required for FFmpeg FFI
- All unsafe blocks must have `// SAFETY:` comments
- Hardware acceleration code is the primary source of unsafe (device context management, frame transfer)

### Feature Flags

```toml
# ffmpeg-source
[features]
default = ["file"]
file = []
http = ["reqwest", "tokio"]
hls = ["http"]

# ffmpeg-decode
[features]
default = []
videotoolbox = []  # macOS hardware acceleration
vaapi = []         # Linux hardware acceleration
cuda = []          # NVIDIA hardware acceleration

# ffmpeg-sink
[features]
default = ["file"]
file = []
hls = []
```

### Seek/Flush Signaling

When a seek occurs:

1. `Source::seek()` seeks in the container
2. Consumer must call `Decoder::reset()` to clear internal buffers
3. First packet after seek is a keyframe (FFmpeg seeks to keyframes)

The `PipelineSignal::Flush` type in `ffmpeg-types` can be used by consumers to propagate flush signals through their pipeline.

### Clock/Sync Architecture

The clock system handles A/V synchronization:

```
┌─────────────────┐
│  AudioConsumer  │───► samples consumed ───►┌─────────────┐
│  (audio output) │                          │ AudioClock  │
└─────────────────┘                          │             │
                                             │ position()  │◄─── Video renderer queries
┌─────────────────┐                          └─────────────┘     to know which frame
│  VideoRenderer  │◄── "current time" ───────────────┘           to display
└─────────────────┘
```

- `AudioClock` is the source of truth when audio is playing
- Audio consumer calls `add_samples()` as it consumes samples
- Video renderer calls `position()` to get current playback time
- When audio ends, `AudioClock::mark_finished()` switches to wall-time extrapolation
- `WallClock` is used for videos without audio

---

## 5. Implementation Phases

### Phase 1: Foundation

- `ffmpeg-types` — all shared types, clock implementations
- Basic tests for timestamp conversion, clock behavior

### Phase 2: Decode Path (enables playback)

- `ffmpeg-source` — file support only (no HTTP yet)
- `ffmpeg-decode` — software decoding only
- `ffmpeg-transform` — video scaling to BGRA, audio to F32 stereo
- Integration test: decode a file, verify frames

### Phase 3: Hardware Acceleration

- `ffmpeg-decode` — VideoToolbox support (macOS)
- Other platforms as needed

### Phase 4: Streaming Input

- `ffmpeg-source` — HTTP support
- `ffmpeg-source` — HLS support
- Requires async infrastructure

### Phase 5: Encode Path (enables transcoding)

- `ffmpeg-encode` — H.264, AAC encoding
- `ffmpeg-sink` — file output (MP4)
- Integration test: transcode a file

### Phase 6: Streaming Output

- `ffmpeg-sink` — HLS output

---

## 6. Migration Path

### From `vidwall` Codebase

| vidwall file                     | Target crate                                         | Notes                               |
| -------------------------------- | ---------------------------------------------------- | ----------------------------------- |
| `src/decode/decoder.rs`          | `ffmpeg-source`, `ffmpeg-decode`, `ffmpeg-transform` | Split into three crates             |
| `src/decode/packet_queue.rs`     | Consumer code (not in crates)                        | Queue pattern stays in app          |
| `src/playback/frame.rs`          | `ffmpeg-types`                                       | `VideoFrame` moves here             |
| `src/audio/stream.rs`            | `ffmpeg-types`                                       | `AudioStreamClock` → `AudioClock`   |
| `src/playback/video_pipeline.rs` | Consumer code                                        | Pipeline orchestration stays in app |
| `src/playback/audio_pipeline.rs` | Consumer code                                        | Pipeline orchestration stays in app |

### What Needs to Be Written From Scratch

- `ffmpeg-encode` — no existing code in vidwall
- `ffmpeg-sink` — no existing code in vidwall
- HTTP/HLS support in `ffmpeg-source`
- Async wrappers

### Key Extractions

**`VideoFrame` and `AudioFrame`** → `ffmpeg-types`

- Add `format` field (currently hardcoded to BGRA)
- Add `time_base` field

**`AudioStreamClock`** → `ffmpeg-types::AudioClock`

- Already well-designed for this purpose
- `FinishedState` pattern for wall-time extrapolation is good

**Decoder functions** → Split across crates:

- `get_video_info()`, `get_audio_stream_info()` → `ffmpeg-source::probe()`
- `video_demux()`, `audio_demux()` → `ffmpeg-source::Source`
- `decode_video_packets()`, `decode_audio_packets()` → `ffmpeg-decode`
- Scaler/resampler logic → `ffmpeg-transform`

---

## 7. Open Questions

### Stream Routing

**Question:** Should `ffmpeg-source` produce a single packet stream that the consumer filters, or separate video/audio iterators?

**Current vidwall approach:** Separate `video_demux()` and `audio_demux()` functions, each opening their own file handle.

**Options:**

1. Single `Source` with `next_packet()` returning tagged packets — consumer routes
2. `Source` with separate `next_video_packet()` / `next_audio_packet()` methods
3. `Source::split()` returning `(VideoPacketStream, AudioPacketStream)`

**Recommendation:** Option 1 (single stream) is simplest and most flexible. Consumer can route packets as needed.

### Hardware Acceleration Context Lifetime

**Question:** Should HW device contexts be created per-decoder or shared?

**Current vidwall approach:** Created per decoder in `create_hw_device_ctx()`.

**Options:**

1. Per-decoder (current) — simple, no sharing complexity
2. Shared via `HwContext` type passed to decoder — more efficient for multiple decoders

**Recommendation:** Start with per-decoder. Add shared context support if profiling shows benefit.

### Codec Parameters Handling

**Question:** How to pass codec parameters from source to decoder?

**Current vidwall approach:** `codec::Parameters` from ffmpeg-next is cloned and passed around.

**Challenge:** We want to avoid exposing ffmpeg-next types.

**Options:**

1. Serialize codec parameters to bytes in `ffmpeg-types`
2. Keep `VideoStreamInfo`/`AudioStreamInfo` rich enough to recreate decoder context
3. Return an opaque `CodecConfig` from source that decoder accepts

**Recommendation:** Option 3 — `ffmpeg-source` returns `CodecConfig` (opaque to consumer), `ffmpeg-decode` accepts it.
