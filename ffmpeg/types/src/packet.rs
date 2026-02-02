/*!
    Encoded packet type.
*/

use crate::{MediaDuration, Pts, Rational};

/**
    Type of media stream.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamType {
    /// Video stream
    Video,
    /// Audio stream
    Audio,
}

/**
    An encoded media packet.

    Contains compressed data from a single stream, with timing information.
    Packets are the unit of data between demuxer and decoder, or between
    encoder and muxer.
*/
#[derive(Clone, Debug)]
pub struct Packet {
    /// Compressed data.
    pub data: Vec<u8>,
    /// Presentation timestamp (when to display/play).
    pub pts: Option<Pts>,
    /// Decode timestamp (when to decode — may differ from PTS for B-frames).
    pub dts: Option<Pts>,
    /// Duration of this packet's content.
    pub duration: MediaDuration,
    /// Time base for interpreting timestamps.
    pub time_base: Rational,
    /// Whether this is a keyframe (can be decoded independently).
    pub is_keyframe: bool,
    /// Type of stream this packet belongs to.
    pub stream_type: StreamType,
}

impl Packet {
    /**
        Create a new packet.
    */
    pub fn new(
        data: Vec<u8>,
        pts: Option<Pts>,
        dts: Option<Pts>,
        duration: MediaDuration,
        time_base: Rational,
        is_keyframe: bool,
        stream_type: StreamType,
    ) -> Self {
        Self {
            data,
            pts,
            dts,
            duration,
            time_base,
            is_keyframe,
            stream_type,
        }
    }

    /**
        Returns the presentation time as a Duration, if PTS is set.
    */
    pub fn presentation_time(&self) -> Option<std::time::Duration> {
        self.pts.map(|pts| pts.to_duration(self.time_base))
    }

    /**
        Returns the decode time as a Duration, if DTS is set.
    */
    pub fn decode_time(&self) -> Option<std::time::Duration> {
        self.dts.map(|dts| dts.to_duration(self.time_base))
    }

    /**
        Returns the packet duration as a std Duration.
    */
    pub fn packet_duration(&self) -> std::time::Duration {
        self.duration.to_duration(self.time_base)
    }

    /**
        Returns true if this packet contains video data.
    */
    pub fn is_video(&self) -> bool {
        self.stream_type == StreamType::Video
    }

    /**
        Returns true if this packet contains audio data.
    */
    pub fn is_audio(&self) -> bool {
        self.stream_type == StreamType::Audio
    }
}

// Ensure Packet is Send + Sync
static_assertions::assert_impl_all!(Packet: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const TB_1_1000: Rational = Rational { num: 1, den: 1000 };

    #[test]
    fn packet_construction() {
        let packet = Packet::new(
            vec![0u8; 1000],
            Some(Pts(500)),
            Some(Pts(400)),
            MediaDuration(100),
            TB_1_1000,
            true,
            StreamType::Video,
        );

        assert_eq!(packet.data.len(), 1000);
        assert!(packet.is_keyframe);
        assert!(packet.is_video());
        assert!(!packet.is_audio());
    }

    #[test]
    fn packet_presentation_time() {
        let packet = Packet::new(
            vec![],
            Some(Pts(1500)),
            None,
            MediaDuration(0),
            TB_1_1000,
            false,
            StreamType::Audio,
        );

        assert_eq!(
            packet.presentation_time(),
            Some(Duration::from_millis(1500))
        );
    }

    #[test]
    fn packet_decode_time() {
        let packet = Packet::new(
            vec![],
            Some(Pts(1500)),
            Some(Pts(1400)),
            MediaDuration(0),
            TB_1_1000,
            false,
            StreamType::Video,
        );

        assert_eq!(packet.decode_time(), Some(Duration::from_millis(1400)));
    }

    #[test]
    fn packet_duration() {
        let packet = Packet::new(
            vec![],
            None,
            None,
            MediaDuration(33),
            TB_1_1000,
            false,
            StreamType::Video,
        );

        assert_eq!(packet.packet_duration(), Duration::from_millis(33));
    }

    #[test]
    fn stream_type_checks() {
        let video = Packet::new(
            vec![],
            None,
            None,
            MediaDuration(0),
            TB_1_1000,
            false,
            StreamType::Video,
        );
        let audio = Packet::new(
            vec![],
            None,
            None,
            MediaDuration(0),
            TB_1_1000,
            false,
            StreamType::Audio,
        );

        assert!(video.is_video());
        assert!(!video.is_audio());
        assert!(audio.is_audio());
        assert!(!audio.is_video());
    }
}
