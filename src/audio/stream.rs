use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer, Split},
};

/// Atomic f32 wrapper for thread-safe volume control
pub struct AtomicF32 {
    inner: AtomicU32,
}

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self {
            inner: AtomicU32::new(value.to_bits()),
        }
    }

    pub fn load(&self, ordering: Ordering) -> f32 {
        f32::from_bits(self.inner.load(ordering))
    }

    pub fn store(&self, value: f32, ordering: Ordering) {
        self.inner.store(value.to_bits(), ordering);
    }
}

/// Default ring buffer size (~2 seconds of stereo audio at 48kHz)
const RING_BUFFER_SIZE: usize = 48000 * 2 * 2;

/// Producer half of the audio stream (used by decoder thread)
///
/// SAFETY: This is safe because ringbuf's HeapProd is designed to be used
/// from a single producer thread while a consumer operates on the other half.
/// The producer and consumer halves can operate independently without locking.
pub struct AudioStreamProducer {
    producer: UnsafeCell<ringbuf::HeapProd<f32>>,
    closed: AtomicBool,
}

// SAFETY: HeapProd is safe to send between threads.
// Only one thread should use the producer at a time (the decoder thread).
unsafe impl Send for AudioStreamProducer {}
unsafe impl Sync for AudioStreamProducer {}

impl AudioStreamProducer {
    /// Push samples to the ring buffer. Returns number of samples written.
    /// This is lock-free and will not block.
    pub fn push(&self, samples: &[f32]) -> usize {
        // SAFETY: Only one thread (decoder) calls push, and ringbuf's
        // producer is designed to work independently from consumer.
        unsafe { (*self.producer.get()).push_slice(samples) }
    }

    /// Check if there's space for more samples
    pub fn available(&self) -> usize {
        // SAFETY: vacant_len() only reads atomic state
        unsafe { (*self.producer.get()).vacant_len() }
    }

    /// Close the producer (signals end of stream)
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Check if closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
}

/// Consumer half of the audio stream (used by audio callback)
///
/// SAFETY: This is safe because ringbuf's HeapCons is designed to be used
/// from a single consumer thread while a producer operates on the other half.
pub struct AudioStreamConsumer {
    consumer: UnsafeCell<ringbuf::HeapCons<f32>>,
    volume: AtomicF32,
    closed: AtomicBool,
}

// SAFETY: HeapCons is safe to send between threads.
// Only one thread should use the consumer at a time (the audio callback thread).
unsafe impl Send for AudioStreamConsumer {}
unsafe impl Sync for AudioStreamConsumer {}

impl AudioStreamConsumer {
    /// Get current volume (0.0 to 1.0)
    pub fn volume(&self) -> f32 {
        self.volume.load(Ordering::Relaxed)
    }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) {
        self.volume.store(volume.clamp(0.0, 1.0), Ordering::Relaxed);
    }

    /// Check if the stream has ended
    pub fn is_ended(&self) -> bool {
        // SAFETY: is_empty() only reads atomic state
        unsafe { self.closed.load(Ordering::Acquire) && (*self.consumer.get()).is_empty() }
    }

    /// Mark as closed (called when producer signals end)
    pub fn mark_closed(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Check how many samples are available
    pub fn available(&self) -> usize {
        // SAFETY: occupied_len() only reads atomic state
        unsafe { (*self.consumer.get()).occupied_len() }
    }

    /// Fill the output buffer with samples, applying volume.
    /// This is completely lock-free and safe for real-time audio.
    ///
    /// Returns: Number of samples actually written
    pub fn fill_buffer(&self, output: &mut [f32]) -> usize {
        let volume = self.volume();

        // SAFETY: Only one thread (audio callback) calls fill_buffer, and ringbuf's
        // consumer is designed to work independently from producer.
        let available = unsafe { (*self.consumer.get()).occupied_len() };
        let to_read = output.len().min(available);

        if to_read > 0 {
            // Read samples from ring buffer
            let read = unsafe { (*self.consumer.get()).pop_slice(&mut output[..to_read]) };

            // Apply volume to the samples we read
            for sample in &mut output[..read] {
                *sample *= volume;
            }

            // Fill remaining with silence
            for sample in &mut output[read..] {
                *sample = 0.0;
            }

            read
        } else {
            // No samples available, output silence
            for sample in output.iter_mut() {
                *sample = 0.0;
            }
            0
        }
    }
}

/// Create a new audio stream pair (producer for decoder, consumer for playback)
pub fn create_audio_stream() -> (AudioStreamProducer, AudioStreamConsumer) {
    let rb = HeapRb::<f32>::new(RING_BUFFER_SIZE);
    let (producer, consumer) = rb.split();

    (
        AudioStreamProducer {
            producer: UnsafeCell::new(producer),
            closed: AtomicBool::new(false),
        },
        AudioStreamConsumer {
            consumer: UnsafeCell::new(consumer),
            volume: AtomicF32::new(1.0),
            closed: AtomicBool::new(false),
        },
    )
}
