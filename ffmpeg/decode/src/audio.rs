/*!
    Audio decoder implementation.
*/

use ffmpeg_next::{
    codec::{self, decoder::Audio as AudioDecoderFFmpeg},
    ffi,
    packet::Mut as PacketMut,
    util::frame::audio::Audio as AudioFrameFFmpeg,
};

use ffmpeg_source::CodecConfig;
use ffmpeg_types::{AudioFrame, ChannelLayout, Error, Packet, Pts, Rational, Result, SampleFormat};

use crate::config::AudioDecoderConfig;

/**
    Audio decoder.

    Decodes audio packets into frames.
*/
pub struct AudioDecoder {
    decoder: AudioDecoderFFmpeg,
    time_base: Rational,
}

impl AudioDecoder {
    /**
        Create a new audio decoder from codec configuration.

        # Arguments

        * `codec_config` - Codec configuration from the source
        * `time_base` - Time base for the audio stream
        * `_config` - Decoder configuration (reserved for future use)
    */
    pub fn new(
        codec_config: CodecConfig,
        time_base: Rational,
        _config: AudioDecoderConfig,
    ) -> Result<Self> {
        ffmpeg_next::init().map_err(|e| Error::codec(e.to_string()))?;

        let parameters = codec_config.into_parameters();

        let decoder_ctx = codec::context::Context::from_parameters(parameters)
            .map_err(|e| Error::codec(e.to_string()))?;

        let decoder = decoder_ctx
            .decoder()
            .audio()
            .map_err(|e| Error::codec(e.to_string()))?;

        Ok(Self { decoder, time_base })
    }

    /**
        Get the time base for this decoder.
    */
    pub fn time_base(&self) -> Rational {
        self.time_base
    }

    /**
        Get the sample rate of the decoded audio.
    */
    pub fn sample_rate(&self) -> u32 {
        self.decoder.rate()
    }

    /**
        Get the number of channels.
    */
    pub fn channels(&self) -> u16 {
        self.decoder.channels() as u16
    }

    /**
        Decode a packet, returning decoded frames.

        May return zero, one, or multiple frames depending on codec.
    */
    pub fn decode(&mut self, packet: &Packet) -> Result<Vec<AudioFrame>> {
        // Create FFmpeg packet from our packet
        let mut ffmpeg_pkt = if packet.data.is_empty() {
            ffmpeg_next::Packet::empty()
        } else {
            ffmpeg_next::Packet::copy(&packet.data)
        };

        // Set timing info
        unsafe {
            let pkt_ptr = ffmpeg_pkt.as_mut_ptr();
            if let Some(pts) = packet.pts {
                (*pkt_ptr).pts = pts.0;
            }
            if let Some(dts) = packet.dts {
                (*pkt_ptr).dts = dts.0;
            }
            (*pkt_ptr).duration = packet.duration.0;
        }

        // Send packet to decoder
        self.decoder
            .send_packet(&ffmpeg_pkt)
            .map_err(|e| Error::codec(e.to_string()))?;

        // Receive all available frames
        self.receive_frames()
    }

    /**
        Flush the decoder to get any remaining buffered frames.

        Call this at end of stream to retrieve any buffered frames.
    */
    pub fn flush(&mut self) -> Result<Vec<AudioFrame>> {
        self.decoder
            .send_eof()
            .map_err(|e| Error::codec(e.to_string()))?;

        self.receive_frames()
    }

    /**
        Reset the decoder after a seek.

        Clears internal buffers. Call this after seeking.
    */
    pub fn reset(&mut self) {
        self.decoder.flush();
    }

    /**
        Receive all available frames from the decoder.
    */
    fn receive_frames(&mut self) -> Result<Vec<AudioFrame>> {
        let mut frames = Vec::new();
        let mut decoded_frame = AudioFrameFFmpeg::empty();

        loop {
            match self.decoder.receive_frame(&mut decoded_frame) {
                Ok(()) => match self.convert_frame(&decoded_frame) {
                    Ok(frame) => frames.push(frame),
                    Err(e) => {
                        eprintln!("[audio_decode] frame conversion error: {}", e);
                    }
                },
                Err(ffmpeg_next::Error::Other { errno }) if errno == ffi::AVERROR(ffi::EAGAIN) => {
                    break;
                }
                Err(ffmpeg_next::Error::Eof) => {
                    break;
                }
                Err(e) => {
                    return Err(Error::codec(e.to_string()));
                }
            }
        }

        Ok(frames)
    }

    /**
        Convert an FFmpeg audio frame to our AudioFrame type.
    */
    fn convert_frame(&self, frame: &AudioFrameFFmpeg) -> Result<AudioFrame> {
        let samples = frame.samples();
        let sample_rate = frame.rate();
        let channel_count = frame.channels() as u16;

        if samples == 0 {
            return Err(Error::invalid_data("audio frame has zero samples"));
        }

        // Get format
        let ffmpeg_format = frame.format();
        let format = sample_format_from_ffmpeg(ffmpeg_format).ok_or_else(|| {
            Error::unsupported_format(format!("unsupported sample format: {:?}", ffmpeg_format))
        })?;

        // Determine channel layout
        let channels = match channel_count {
            1 => ChannelLayout::Mono,
            _ => ChannelLayout::Stereo,
        };

        // Get PTS
        let pts = frame.pts().map(Pts);

        // Copy frame data
        let data = copy_audio_data(frame, format, samples, channel_count)?;

        Ok(AudioFrame::new(
            data,
            samples,
            sample_rate,
            channels,
            format,
            pts,
            self.time_base,
        ))
    }
}

/**
    Copy audio data from FFmpeg frame.
*/
fn copy_audio_data(
    frame: &AudioFrameFFmpeg,
    format: SampleFormat,
    samples: usize,
    channels: u16,
) -> Result<Vec<u8>> {
    let bytes_per_sample = format.bytes_per_sample();
    let is_planar = frame.is_planar();

    if is_planar {
        // Planar format - interleave the channels
        let total_bytes = samples * channels as usize * bytes_per_sample;
        let mut output = vec![0u8; total_bytes];

        for ch in 0..channels as usize {
            let plane_data = frame.data(ch);
            for s in 0..samples {
                let src_offset = s * bytes_per_sample;
                let dst_offset = (s * channels as usize + ch) * bytes_per_sample;
                output[dst_offset..dst_offset + bytes_per_sample]
                    .copy_from_slice(&plane_data[src_offset..src_offset + bytes_per_sample]);
            }
        }

        Ok(output)
    } else {
        // Packed/interleaved format - just copy
        let plane_data = frame.data(0);
        let total_bytes = samples * channels as usize * bytes_per_sample;
        Ok(plane_data[..total_bytes].to_vec())
    }
}

/**
    Convert FFmpeg sample format to our SampleFormat.
*/
fn sample_format_from_ffmpeg(format: ffmpeg_next::format::Sample) -> Option<SampleFormat> {
    use ffmpeg_next::format::Sample;

    match format {
        Sample::F32(_) => Some(SampleFormat::F32),
        Sample::F64(_) => Some(SampleFormat::F64),
        Sample::I16(_) => Some(SampleFormat::S16),
        Sample::I32(_) => Some(SampleFormat::S32),
        Sample::U8(_) => Some(SampleFormat::U8),
        _ => None,
    }
}

impl std::fmt::Debug for AudioDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioDecoder")
            .field("time_base", &self.time_base)
            .field("sample_rate", &self.decoder.rate())
            .field("channels", &self.decoder.channels())
            .finish_non_exhaustive()
    }
}
