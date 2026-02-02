/*!
    Timestamp types for media timing.
*/

use std::time::Duration;

use crate::Rational;

/**
    Presentation timestamp in time_base units.

    This is the raw timestamp value from the media stream. To convert to
    a meaningful duration, you need the stream's time base.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pts(pub i64);

impl Pts {
    /**
        Convert this PTS to a Duration using the given time base.

        Negative PTS values are clamped to zero.
    */
    #[inline]
    pub fn to_duration(self, time_base: Rational) -> Duration {
        if self.0 <= 0 {
            return Duration::ZERO;
        }
        let seconds = self.0 as f64 * time_base.to_f64();
        Duration::from_secs_f64(seconds.max(0.0))
    }

    /**
        Create a PTS from a Duration using the given time base.
    */
    #[inline]
    pub fn from_duration(duration: Duration, time_base: Rational) -> Self {
        let seconds = duration.as_secs_f64();
        let pts = (seconds / time_base.to_f64()).round() as i64;
        Self(pts)
    }
}

impl From<i64> for Pts {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<Pts> for i64 {
    fn from(pts: Pts) -> Self {
        pts.0
    }
}

/**
    Duration in time_base units.

    Similar to Pts but semantically represents a duration rather than a point in time.
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaDuration(pub i64);

impl MediaDuration {
    /**
        Convert this duration to a std Duration using the given time base.

        Negative values are clamped to zero.
    */
    #[inline]
    pub fn to_duration(self, time_base: Rational) -> Duration {
        if self.0 <= 0 {
            return Duration::ZERO;
        }
        let seconds = self.0 as f64 * time_base.to_f64();
        Duration::from_secs_f64(seconds.max(0.0))
    }

    /**
        Create a MediaDuration from a std Duration using the given time base.
    */
    #[inline]
    pub fn from_duration(duration: Duration, time_base: Rational) -> Self {
        let seconds = duration.as_secs_f64();
        let ticks = (seconds / time_base.to_f64()).round() as i64;
        Self(ticks)
    }
}

impl From<i64> for MediaDuration {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<MediaDuration> for i64 {
    fn from(duration: MediaDuration) -> Self {
        duration.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TB_1_1000: Rational = Rational { num: 1, den: 1000 };
    const TB_1_90000: Rational = Rational { num: 1, den: 90000 };
    const TB_1_48000: Rational = Rational { num: 1, den: 48000 };

    #[test]
    fn pts_to_duration_milliseconds() {
        // 1000 ticks at 1/1000 = 1 second
        let pts = Pts(1000);
        let dur = pts.to_duration(TB_1_1000);
        assert_eq!(dur, Duration::from_secs(1));
    }

    #[test]
    fn pts_to_duration_mpeg_ts() {
        // 90000 ticks at 1/90000 = 1 second
        let pts = Pts(90000);
        let dur = pts.to_duration(TB_1_90000);
        assert_eq!(dur, Duration::from_secs(1));
    }

    #[test]
    fn pts_to_duration_audio() {
        // 48000 ticks at 1/48000 = 1 second
        let pts = Pts(48000);
        let dur = pts.to_duration(TB_1_48000);
        assert_eq!(dur, Duration::from_secs(1));
    }

    #[test]
    fn pts_zero() {
        let pts = Pts(0);
        assert_eq!(pts.to_duration(TB_1_1000), Duration::ZERO);
    }

    #[test]
    fn pts_negative_clamps_to_zero() {
        let pts = Pts(-100);
        assert_eq!(pts.to_duration(TB_1_1000), Duration::ZERO);
    }

    #[test]
    fn pts_from_duration() {
        let dur = Duration::from_secs(1);
        let pts = Pts::from_duration(dur, TB_1_1000);
        assert_eq!(pts.0, 1000);
    }

    #[test]
    fn pts_round_trip() {
        let original = Duration::from_millis(1500);
        let pts = Pts::from_duration(original, TB_1_1000);
        let back = pts.to_duration(TB_1_1000);
        assert_eq!(back, original);
    }

    #[test]
    fn media_duration_to_duration() {
        let md = MediaDuration(2000);
        let dur = md.to_duration(TB_1_1000);
        assert_eq!(dur, Duration::from_secs(2));
    }

    #[test]
    fn media_duration_from_duration() {
        let dur = Duration::from_secs(2);
        let md = MediaDuration::from_duration(dur, TB_1_1000);
        assert_eq!(md.0, 2000);
    }

    #[test]
    fn pts_ordering() {
        assert!(Pts(100) < Pts(200));
        assert!(Pts(200) > Pts(100));
        assert_eq!(Pts(100), Pts(100));
    }

    #[test]
    fn media_duration_negative_clamps_to_zero() {
        let md = MediaDuration(-50);
        assert_eq!(md.to_duration(TB_1_1000), Duration::ZERO);
    }
}
