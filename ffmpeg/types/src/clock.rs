/*!
    Clock and synchronization types.
*/

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/**
    Trait for playback clocks.

    A clock provides the current playback position, which is used for A/V
    synchronization. Video renderers query the clock to determine which
    frame to display.
*/
pub trait Clock: Send + Sync {
    /// Get the current playback position.
    fn position(&self) -> Duration;

    /// Reset the clock to a specific position (e.g., after seeking).
    fn reset_to(&self, position: Duration);
}

/**
    Audio-driven clock that tracks position based on samples consumed.

    This is the primary clock for videos with audio. The audio subsystem
    calls `add_samples()` as it consumes samples, and video uses `position()`
    to determine which frame to display.

    When audio playback finishes (all samples consumed and stream closed),
    the clock automatically switches to wall-time extrapolation so video
    frames continue to advance correctly.
*/
pub struct AudioClock {
    /**
        Total samples consumed (interleaved, so L+R = 2 samples for stereo).
    */
    samples_consumed: AtomicU64,
    /**
        Sample rate in Hz.
    */
    sample_rate: u32,
    /**
        Number of channels.
    */
    channels: u16,
    /**
        State when audio finishes — protected by a simple atomic flag + instant.
        We use atomics to avoid locks in the audio callback path.
    */
    finished_at_samples: AtomicU64,
    finished_at_instant: std::sync::Mutex<Option<Instant>>,
}

impl AudioClock {
    /**
        Create a new audio clock with the given sample rate and channel count.
    */
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            samples_consumed: AtomicU64::new(0),
            sample_rate,
            channels,
            finished_at_samples: AtomicU64::new(u64::MAX), // sentinel for "not finished"
            finished_at_instant: std::sync::Mutex::new(None),
        }
    }

    /**
        Add consumed samples to the clock.

        Called by the audio consumer as it outputs samples.
        `count` is the total number of samples (frames * channels).
    */
    pub fn add_samples(&self, count: u64) {
        self.samples_consumed.fetch_add(count, Ordering::Relaxed);
    }

    /**
        Mark the audio stream as finished.

        After this, `position()` will extrapolate using wall time from
        the point where audio ended.
    */
    pub fn mark_finished(&self) {
        let current_samples = self.samples_consumed.load(Ordering::Relaxed);

        // Only set if not already finished
        let _ = self.finished_at_samples.compare_exchange(
            u64::MAX,
            current_samples,
            Ordering::SeqCst,
            Ordering::Relaxed,
        );

        let mut guard = self.finished_at_instant.lock().unwrap();
        if guard.is_none() {
            *guard = Some(Instant::now());
        }
    }

    /**
        Get the total number of samples consumed.
    */
    pub fn samples_consumed(&self) -> u64 {
        self.samples_consumed.load(Ordering::Relaxed)
    }

    /**
        Get the sample rate.
    */
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /**
        Get the channel count.
    */
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /**
        Calculate position from sample count.
    */
    fn samples_to_duration(&self, samples: u64) -> Duration {
        // Samples are interleaved, so divide by channels to get audio frames
        let audio_frames = samples / self.channels as u64;
        Duration::from_secs_f64(audio_frames as f64 / self.sample_rate as f64)
    }
}

impl Clock for AudioClock {
    fn position(&self) -> Duration {
        let finished_samples = self.finished_at_samples.load(Ordering::Relaxed);

        if finished_samples != u64::MAX {
            // Audio has finished — extrapolate from wall time
            let guard = self.finished_at_instant.lock().unwrap();
            if let Some(finished_instant) = *guard {
                let position_at_finish = self.samples_to_duration(finished_samples);
                let elapsed_since_finish = finished_instant.elapsed();
                return position_at_finish + elapsed_since_finish;
            }
        }

        // Normal case: return position based on samples consumed
        let samples = self.samples_consumed.load(Ordering::Relaxed);
        self.samples_to_duration(samples)
    }

    fn reset_to(&self, position: Duration) {
        // Calculate sample count for this position
        let audio_frames = (position.as_secs_f64() * self.sample_rate as f64) as u64;
        let samples = audio_frames * self.channels as u64;
        self.samples_consumed.store(samples, Ordering::Relaxed);

        // Clear finished state
        self.finished_at_samples.store(u64::MAX, Ordering::Relaxed);
        *self.finished_at_instant.lock().unwrap() = None;
    }
}

// Verify AudioClock is Send + Sync
static_assertions::assert_impl_all!(AudioClock: Send, Sync, Clock);

/**
    Wall-time clock for videos without audio.

    Uses wall time to track playback position. Supports pause and resume.
*/
pub struct WallClock {
    /// When playback started (or was last reset).
    start_instant: std::sync::Mutex<Instant>,
    /// Offset to add to elapsed time (from resets/seeks).
    offset: std::sync::Mutex<Duration>,
    /// When pause started (None if not paused).
    paused_at: std::sync::Mutex<Option<Instant>>,
    /// Total time spent paused (accumulated across multiple pauses).
    paused_duration: std::sync::Mutex<Duration>,
}

impl WallClock {
    /**
        Create a new wall clock starting at position zero.
    */
    pub fn new() -> Self {
        Self {
            start_instant: std::sync::Mutex::new(Instant::now()),
            offset: std::sync::Mutex::new(Duration::ZERO),
            paused_at: std::sync::Mutex::new(None),
            paused_duration: std::sync::Mutex::new(Duration::ZERO),
        }
    }

    /**
        Pause the clock.

        While paused, `position()` returns the same value.
    */
    pub fn pause(&self) {
        let mut paused_at = self.paused_at.lock().unwrap();
        if paused_at.is_none() {
            *paused_at = Some(Instant::now());
        }
    }

    /**
        Resume the clock after being paused.
    */
    pub fn resume(&self) {
        let mut paused_at = self.paused_at.lock().unwrap();
        if let Some(pause_start) = paused_at.take() {
            let mut paused_duration = self.paused_duration.lock().unwrap();
            *paused_duration += pause_start.elapsed();
        }
    }

    /**
        Check if the clock is paused.
    */
    pub fn is_paused(&self) -> bool {
        self.paused_at.lock().unwrap().is_some()
    }
}

impl Default for WallClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for WallClock {
    fn position(&self) -> Duration {
        let start = *self.start_instant.lock().unwrap();
        let offset = *self.offset.lock().unwrap();
        let paused_duration = *self.paused_duration.lock().unwrap();
        let paused_at = *self.paused_at.lock().unwrap();

        if let Some(pause_start) = paused_at {
            // Currently paused — return position at pause time
            let elapsed_before_pause = pause_start.duration_since(start);
            offset + elapsed_before_pause - paused_duration
        } else {
            // Not paused — return current position
            let elapsed = start.elapsed();
            offset + elapsed - paused_duration
        }
    }

    fn reset_to(&self, position: Duration) {
        *self.start_instant.lock().unwrap() = Instant::now();
        *self.offset.lock().unwrap() = position;
        *self.paused_at.lock().unwrap() = None;
        *self.paused_duration.lock().unwrap() = Duration::ZERO;
    }
}

// Verify WallClock is Send + Sync
static_assertions::assert_impl_all!(WallClock: Send, Sync, Clock);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_clock_initial_position() {
        let clock = AudioClock::new(48000, 2);
        assert_eq!(clock.position(), Duration::ZERO);
    }

    #[test]
    fn audio_clock_add_samples() {
        let clock = AudioClock::new(48000, 2);

        // Add 1 second of stereo audio (48000 frames * 2 channels)
        clock.add_samples(48000 * 2);

        let pos = clock.position();
        assert_eq!(pos, Duration::from_secs(1));
    }

    #[test]
    fn audio_clock_add_samples_fractional() {
        let clock = AudioClock::new(48000, 2);

        // Add 0.5 seconds of stereo audio
        clock.add_samples(24000 * 2);

        let pos = clock.position();
        assert_eq!(pos, Duration::from_millis(500));
    }

    #[test]
    fn audio_clock_reset_to() {
        let clock = AudioClock::new(48000, 2);

        // Add some samples
        clock.add_samples(48000 * 2);
        assert_eq!(clock.position(), Duration::from_secs(1));

        // Reset to 5 seconds
        clock.reset_to(Duration::from_secs(5));
        assert_eq!(clock.position(), Duration::from_secs(5));
    }

    #[test]
    fn audio_clock_mono() {
        let clock = AudioClock::new(48000, 1);

        // Add 1 second of mono audio
        clock.add_samples(48000);

        assert_eq!(clock.position(), Duration::from_secs(1));
    }

    #[test]
    fn audio_clock_samples_consumed() {
        let clock = AudioClock::new(48000, 2);
        clock.add_samples(1000);
        assert_eq!(clock.samples_consumed(), 1000);
    }

    #[test]
    fn audio_clock_mark_finished_extrapolates() {
        let clock = AudioClock::new(48000, 2);

        // Add 1 second of audio
        clock.add_samples(48000 * 2);

        // Mark as finished
        clock.mark_finished();

        // Position should be at least 1 second (plus any wall time elapsed)
        let pos = clock.position();
        assert!(pos >= Duration::from_secs(1));
    }

    #[test]
    fn audio_clock_reset_clears_finished() {
        let clock = AudioClock::new(48000, 2);
        clock.add_samples(48000 * 2);
        clock.mark_finished();

        // Reset to 0
        clock.reset_to(Duration::ZERO);

        // Add more samples — should work normally again
        clock.add_samples(24000 * 2);
        assert_eq!(clock.position(), Duration::from_millis(500));
    }

    // WallClock tests

    #[test]
    fn wall_clock_initial_position() {
        let clock = WallClock::new();
        // Should be very close to zero
        assert!(clock.position() < Duration::from_millis(10));
    }

    #[test]
    fn wall_clock_advances() {
        let clock = WallClock::new();
        std::thread::sleep(Duration::from_millis(50));

        let pos = clock.position();
        // Allow tolerance for scheduling
        assert!(pos >= Duration::from_millis(30));
        assert!(pos < Duration::from_millis(200));
    }

    #[test]
    fn wall_clock_pause_stops_advancement() {
        let clock = WallClock::new();
        std::thread::sleep(Duration::from_millis(50));

        clock.pause();
        let pos_at_pause = clock.position();

        std::thread::sleep(Duration::from_millis(50));

        // Position should not have advanced (much) while paused
        let pos_after_sleep = clock.position();
        let diff = if pos_after_sleep > pos_at_pause {
            pos_after_sleep - pos_at_pause
        } else {
            pos_at_pause - pos_after_sleep
        };
        assert!(diff < Duration::from_millis(10));
    }

    #[test]
    fn wall_clock_resume_continues() {
        let clock = WallClock::new();
        std::thread::sleep(Duration::from_millis(50));

        clock.pause();
        let pos_at_pause = clock.position();

        std::thread::sleep(Duration::from_millis(50));
        clock.resume();

        std::thread::sleep(Duration::from_millis(50));

        // Position should have advanced by ~50ms since resume, not 100ms
        let pos_now = clock.position();
        let total_advance = pos_now - pos_at_pause;
        assert!(total_advance >= Duration::from_millis(30));
        assert!(total_advance < Duration::from_millis(150));
    }

    #[test]
    fn wall_clock_reset_to() {
        let clock = WallClock::new();
        std::thread::sleep(Duration::from_millis(50));

        clock.reset_to(Duration::from_secs(10));

        // Should be close to 10 seconds now
        let pos = clock.position();
        assert!(pos >= Duration::from_secs(10));
        assert!(pos < Duration::from_millis(10100));
    }

    #[test]
    fn wall_clock_is_paused() {
        let clock = WallClock::new();
        assert!(!clock.is_paused());

        clock.pause();
        assert!(clock.is_paused());

        clock.resume();
        assert!(!clock.is_paused());
    }

    #[test]
    fn wall_clock_default() {
        let clock = WallClock::default();
        assert!(clock.position() < Duration::from_millis(10));
    }
}
