/*!
    Rational number type for time bases and frame rates.
*/

use std::fmt;

/**
    A rational number represented as a numerator and denominator.

    Used for time bases (e.g., 1/90000 for MPEG-TS) and frame rates
    (e.g., 24000/1001 for 23.976 fps).
*/
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    pub num: i32,
    pub den: i32,
}

impl Rational {
    /**
        Create a new rational number.

        # Panics

        Panics if `den` is zero.
    */
    #[inline]
    pub const fn new(num: i32, den: i32) -> Self {
        assert!(den != 0, "denominator cannot be zero");
        Self { num, den }
    }

    /**
        Convert to f64.
    */
    #[inline]
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /**
        Invert the rational (swap numerator and denominator).

        # Panics

        Panics if numerator is zero.
    */
    #[inline]
    pub const fn invert(self) -> Self {
        assert!(self.num != 0, "cannot invert zero");
        Self {
            num: self.den,
            den: self.num,
        }
    }
}

impl fmt::Debug for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl From<(i32, i32)> for Rational {
    fn from((num, den): (i32, i32)) -> Self {
        Self::new(num, den)
    }
}

impl From<i32> for Rational {
    fn from(num: i32) -> Self {
        Self::new(num, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rational() {
        let r = Rational::new(1, 1000);
        assert_eq!(r.num, 1);
        assert_eq!(r.den, 1000);
    }

    #[test]
    #[should_panic(expected = "denominator cannot be zero")]
    fn zero_denominator_panics() {
        Rational::new(1, 0);
    }

    #[test]
    fn to_f64_conversion() {
        assert_eq!(Rational::new(1, 2).to_f64(), 0.5);
        assert_eq!(Rational::new(1, 1000).to_f64(), 0.001);
        assert_eq!(Rational::new(24000, 1001).to_f64(), 24000.0 / 1001.0);
    }

    #[test]
    fn invert() {
        let r = Rational::new(1, 90000);
        let inv = r.invert();
        assert_eq!(inv.num, 90000);
        assert_eq!(inv.den, 1);
    }

    #[test]
    #[should_panic(expected = "cannot invert zero")]
    fn invert_zero_panics() {
        Rational::new(0, 1).invert();
    }

    #[test]
    fn from_tuple() {
        let r: Rational = (30000, 1001).into();
        assert_eq!(r.num, 30000);
        assert_eq!(r.den, 1001);
    }

    #[test]
    fn from_i32() {
        let r: Rational = 25.into();
        assert_eq!(r.num, 25);
        assert_eq!(r.den, 1);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Rational::new(1, 90000)), "1/90000");
    }
}
