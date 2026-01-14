use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use gpui::RenderImage;
use image::{Frame, RgbaImage};

use super::decoder::{
    DecoderError, decode_audio_packets, decode_video_packets, demux, get_stream_info,
    get_video_info,
};
use super::frame::VideoFrame;
use super::packet_queue::PacketQueue;
use super::queue::FrameQueue;
use crate::audio::{
    AudioStreamClock, AudioStreamConsumer, AudioStreamProducer, create_audio_stream,
};

const DEFAULT_VIDEO_QUEUE_CAPACITY: usize = 60;
const VIDEO_PACKET_QUEUE_CAPACITY: usize = 120;
const AUDIO_PACKET_QUEUE_CAPACITY: usize = 240;

/**
    Playback state
*/
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Ended,
    Error,
}

/**
    High-level video player that manages decoding and playback timing.

    Uses three threads for decoding:
    - Demux thread: reads packets from file, routes to packet queues
    - Video decode thread: decodes video packets to frames
    - Audio decode thread: decodes audio packets to samples

    This architecture prevents video blocking from starving audio decoding.
*/
pub struct VideoPlayer {
    path: PathBuf,
    // Output queues
    video_queue: Arc<FrameQueue>,
    // Audio components
    audio_producer: Option<Arc<AudioStreamProducer>>,
    audio_consumer: Option<Arc<AudioStreamConsumer>>,
    audio_clock: Option<Arc<AudioStreamClock>>,
    // Packet queues for inter-thread communication
    video_packet_queue: Arc<PacketQueue>,
    audio_packet_queue: Option<Arc<PacketQueue>>,
    // Thread handles
    stop_flag: Arc<AtomicBool>,
    demux_handle: Option<JoinHandle<Result<(), DecoderError>>>,
    video_decode_handle: Option<JoinHandle<Result<(), DecoderError>>>,
    audio_decode_handle: Option<JoinHandle<Result<(), DecoderError>>>,
    // Timing
    start_time: Instant,
    // Frame state
    current_frame: Mutex<Option<VideoFrame>>,
    next_frame: Mutex<Option<VideoFrame>>,
    base_pts: Mutex<Option<Duration>>,
    duration: Duration,
    state: Mutex<PlaybackState>,
    // Render cache
    cached_render_image: Mutex<Option<Arc<RenderImage>>>,
    frame_generation: AtomicU64,
}

impl VideoPlayer {
    /**
        Create a new video player for the given file
    */
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DecoderError> {
        Self::with_options(path, None, None)
    }

    /**
        Create a new video player with target dimensions
    */
    pub fn with_options<P: AsRef<Path>>(
        path: P,
        target_width: Option<u32>,
        target_height: Option<u32>,
    ) -> Result<Self, DecoderError> {
        let path = path.as_ref().to_path_buf();
        let info = get_video_info(&path)?;
        let stream_info = get_stream_info(&path)?;
        let start_time = Instant::now();

        // Create output queues
        let video_queue = Arc::new(FrameQueue::new(DEFAULT_VIDEO_QUEUE_CAPACITY));
        let stop_flag = Arc::new(AtomicBool::new(false));

        // Create packet queues
        let video_packet_queue = Arc::new(PacketQueue::new(VIDEO_PACKET_QUEUE_CAPACITY));
        let audio_packet_queue = if info.has_audio {
            Some(Arc::new(PacketQueue::new(AUDIO_PACKET_QUEUE_CAPACITY)))
        } else {
            None
        };

        // Create audio stream if video has audio
        let (audio_producer, audio_consumer, audio_clock) = if info.has_audio {
            let (producer, consumer, clock) = create_audio_stream();
            (
                Some(Arc::new(producer)),
                Some(Arc::new(consumer)),
                Some(clock),
            )
        } else {
            (None, None, None)
        };

        // Spawn demux thread
        let demux_handle = {
            let path = path.clone();
            let video_pq = Arc::clone(&video_packet_queue);
            let audio_pq = audio_packet_queue.clone();
            let stop = Arc::clone(&stop_flag);
            thread::spawn(move || demux(path, video_pq, audio_pq, stop))
        };

        // Spawn video decode thread
        let video_decode_handle = {
            let packets = Arc::clone(&video_packet_queue);
            let frames = Arc::clone(&video_queue);
            let params = stream_info.video_codec_params.clone();
            let tb = stream_info.video_time_base;
            let stop = Arc::clone(&stop_flag);
            thread::spawn(move || {
                decode_video_packets(
                    packets,
                    frames,
                    params,
                    tb,
                    stop,
                    target_width,
                    target_height,
                )
            })
        };

        // Spawn audio decode thread (if has audio)
        let audio_decode_handle = if let (Some(packets), Some(producer), Some(params), Some(tb)) = (
            audio_packet_queue.clone(),
            audio_producer.clone(),
            stream_info.audio_codec_params.clone(),
            stream_info.audio_time_base,
        ) {
            let stop = Arc::clone(&stop_flag);
            Some(thread::spawn(move || {
                decode_audio_packets(packets, producer, params, tb, stop)
            }))
        } else {
            None
        };

        Ok(Self {
            path,
            video_queue,
            audio_producer,
            audio_consumer,
            audio_clock,
            video_packet_queue,
            audio_packet_queue,
            stop_flag,
            demux_handle: Some(demux_handle),
            video_decode_handle: Some(video_decode_handle),
            audio_decode_handle,
            start_time,
            current_frame: Mutex::new(None),
            next_frame: Mutex::new(None),
            base_pts: Mutex::new(None),
            duration: info.duration,
            state: Mutex::new(PlaybackState::Playing),
            cached_render_image: Mutex::new(None),
            frame_generation: AtomicU64::new(0),
        })
    }

    /**
        Get the video file path
    */
    pub fn path(&self) -> &Path {
        &self.path
    }

    /**
        Get the video duration
    */
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /**
        Get the current playback position.
        For videos with audio, this is the audio clock position.
        For videos without audio, this is based on wall clock.
    */
    pub fn position(&self) -> Duration {
        self.get_playback_time()
    }

    /**
        Get the current playback time to use for frame timing.
        Uses audio clock if available, otherwise wall clock.
    */
    fn get_playback_time(&self) -> Duration {
        if let Some(ref clock) = self.audio_clock {
            clock.position()
        } else {
            self.start_time.elapsed()
        }
    }

    /**
        Get the current playback state
    */
    pub fn state(&self) -> PlaybackState {
        *self.state.lock().unwrap()
    }

    /**
        Check if playback has ended
    */
    pub fn is_ended(&self) -> bool {
        self.state() == PlaybackState::Ended
    }

    /**
        Get the audio stream consumer if this video has audio
    */
    pub fn audio_consumer(&self) -> Option<&Arc<AudioStreamConsumer>> {
        self.audio_consumer.as_ref()
    }

    /**
        Get the audio clock if this video has audio.
    */
    pub fn audio_clock(&self) -> Option<&Arc<AudioStreamClock>> {
        self.audio_clock.as_ref()
    }

    /**
        Set the volume for this video's audio (0.0 to 1.0)
    */
    pub fn set_volume(&self, volume: f32) {
        if let Some(ref consumer) = self.audio_consumer {
            consumer.set_volume(volume);
        }
    }

    /**
        Get the current volume for this video's audio (0.0 to 1.0)
    */
    pub fn volume(&self) -> f32 {
        self.audio_consumer
            .as_ref()
            .map(|c| c.volume())
            .unwrap_or(0.0)
    }

    /**
        Check if this video has an audio track
    */
    pub fn has_audio(&self) -> bool {
        self.audio_consumer.is_some()
    }

    /**
        Get the cached RenderImage for the current frame.
        Only creates a new RenderImage when the frame actually changes.

        For videos with audio, frame timing is driven by the audio clock.
        For videos without audio, frame timing uses wall clock.

        Returns (current_image, old_image_to_drop)
    */
    pub fn get_render_image(&self) -> (Option<Arc<RenderImage>>, Option<Arc<RenderImage>>) {
        let elapsed = self.get_playback_time();

        let mut current = self.current_frame.lock().unwrap();
        let mut next = self.next_frame.lock().unwrap();
        let mut base_pts = self.base_pts.lock().unwrap();
        let mut state = self.state.lock().unwrap();
        let mut cached = self.cached_render_image.lock().unwrap();

        let mut frame_changed = false;
        let mut old_image: Option<Arc<RenderImage>> = None;

        // If we don't have a next frame buffered, try to get one
        if next.is_none() {
            *next = self.video_queue.try_pop();
        }

        // Initialize base_pts from the first frame
        if base_pts.is_none() {
            if let Some(ref frame) = *next {
                *base_pts = Some(frame.pts);
            }
        }

        // Advance to the next frame if its PTS has passed
        if let Some(ref frame) = *next {
            let base = base_pts.unwrap_or(Duration::ZERO);
            let relative_pts = frame.pts.saturating_sub(base);

            if elapsed >= relative_pts {
                *current = next.take();
                frame_changed = true;
                self.frame_generation.fetch_add(1, Ordering::Relaxed);
                *next = self.video_queue.try_pop();
            }
        }

        // Check for end of playback
        if next.is_none() && self.video_queue.is_closed() {
            if current.is_some() {
                let base = base_pts.unwrap_or(Duration::ZERO);
                let adjusted_duration = self.duration.saturating_sub(base);
                if elapsed > adjusted_duration {
                    *state = PlaybackState::Ended;
                }
            }
        }

        // Only create new RenderImage if frame changed or we don't have one yet
        if frame_changed || cached.is_none() {
            if let Some(ref frame) = *current {
                if let Some(render_image) = frame_to_render_image(frame) {
                    old_image = cached.take();
                    *cached = Some(Arc::new(render_image));
                }
            }
        }

        (cached.clone(), old_image)
    }

    /**
        Get the current frame for rendering based on elapsed time.
    */
    pub fn get_frame(&self) -> Option<VideoFrame> {
        self.current_frame.lock().unwrap().clone()
    }

    /**
        Get the number of buffered video frames
    */
    pub fn buffered_frames(&self) -> usize {
        self.video_queue.len()
    }

    /**
        Get the number of buffered audio samples
    */
    pub fn buffered_audio_samples(&self) -> usize {
        self.audio_consumer
            .as_ref()
            .map(|c| c.available())
            .unwrap_or(0)
    }

    /**
        Stop playback and clean up resources
    */
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);

        // Close all queues to unblock threads
        self.video_packet_queue.close();
        if let Some(ref q) = self.audio_packet_queue {
            q.close();
        }
        self.video_queue.close();
        if let Some(ref producer) = self.audio_producer {
            producer.close();
        }

        // Join all threads
        if let Some(handle) = self.demux_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.video_decode_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.audio_decode_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

/**
    Convert a VideoFrame to a RenderImage
*/
fn frame_to_render_image(frame: &VideoFrame) -> Option<RenderImage> {
    let image = RgbaImage::from_raw(frame.width, frame.height, frame.data.clone())?;
    let img_frame = Frame::new(image);
    Some(RenderImage::new(vec![img_frame]))
}
