use std::time::Duration;

use crate::raw::root::RED4ext as red;

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct EngineTime(red::EngineTime);

impl EngineTime {
    pub fn is_valid(&self) -> bool {
        self.0.unk00 != [0; 8]
    }

    pub fn as_secs_f64(&self) -> f64 {
        f64::from_ne_bytes(self.0.unk00)
    }
}

impl std::ops::AddAssign<f64> for EngineTime {
    /// # Panics
    ///
    /// Panics if the sum ends up being `f64::NAN`, `f64::INFINITY` or `f64::NEG_INFINITY`.
    fn add_assign(&mut self, rhs: f64) {
        let current = self.as_secs_f64();
        let addition = current + rhs;
        assert!(!addition.is_infinite(), "EngineTime cannot be infinity");
        assert!(!addition.is_nan(), "EngineTime cannot be NaN");
        self.0.unk00 = addition.to_ne_bytes();
    }
}

impl std::ops::Add<f64> for EngineTime {
    type Output = EngineTime;

    /// # Panics
    ///
    /// Panics if the sum ends up being `f64::NAN`, `f64::INFINITY` or `f64::NEG_INFINITY`.
    fn add(self, rhs: f64) -> Self::Output {
        use std::ops::AddAssign;
        let mut copy = self;
        copy.add_assign(rhs);
        copy
    }
}

impl std::ops::SubAssign<f64> for EngineTime {
    /// # Panics
    ///
    /// Panics if the sum ends up being `f64::NAN`, `f64::INFINITY` or `f64::NEG_INFINITY`.
    fn sub_assign(&mut self, rhs: f64) {
        let current = self.as_secs_f64();
        let substraction = current - rhs;
        assert!(!substraction.is_infinite(), "EngineTime cannot be infinity");
        assert!(!substraction.is_nan(), "EngineTime cannot be NaN");
        self.0.unk00 = substraction.to_ne_bytes();
    }
}

impl std::ops::Sub<f64> for EngineTime {
    type Output = EngineTime;

    /// # Panics
    ///
    /// Panics if the sum ends up being `f64::NAN`, `f64::INFINITY` or `f64::NEG_INFINITY`.
    fn sub(self, rhs: f64) -> Self::Output {
        use std::ops::SubAssign;
        let mut copy = self;
        copy.sub_assign(rhs);
        copy
    }
}

impl PartialEq for EngineTime {
    fn eq(&self, other: &Self) -> bool {
        self.0.unk00.eq(&other.0.unk00)
    }
}

impl PartialOrd for EngineTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.unk00.partial_cmp(&other.0.unk00)
    }
}

impl TryFrom<f64> for EngineTime {
    type Error = EngineTimeError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_infinite() {
            return Err(EngineTimeError::OutOfBounds);
        }
        if value.is_nan() {
            return Err(EngineTimeError::NotANumber);
        }
        Ok(Self(red::EngineTime {
            unk00: value.to_ne_bytes(),
        }))
    }
}

impl From<EngineTime> for f64 {
    fn from(EngineTime(red::EngineTime { unk00 }): EngineTime) -> Self {
        Self::from_ne_bytes(unk00)
    }
}

impl TryFrom<Duration> for EngineTime {
    type Error = EngineTimeError;

    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        let value = value.as_secs_f64();
        value.try_into()
    }
}

impl std::ops::Add<Duration> for EngineTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        self.add(rhs.as_secs_f64())
    }
}

impl std::ops::AddAssign<Duration> for EngineTime {
    fn add_assign(&mut self, rhs: Duration) {
        self.add_assign(rhs.as_secs_f64());
    }
}

impl std::ops::Sub<Duration> for EngineTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        self.sub(rhs.as_secs_f64())
    }
}

impl std::ops::SubAssign<Duration> for EngineTime {
    fn sub_assign(&mut self, rhs: Duration) {
        self.sub_assign(rhs.as_secs_f64());
    }
}

#[derive(Debug)]
pub enum EngineTimeError {
    OutOfBounds,
    NotANumber,
}

impl std::fmt::Display for EngineTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OutOfBounds => "invalid infinite or negative infinite floating-point",
                Self::NotANumber => "invalid NaN",
            }
        )
    }
}

impl std::error::Error for EngineTimeError {}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::EngineTime;

    #[test]
    fn bounds() {
        assert!(EngineTime::try_from(f64::INFINITY).is_err());
        assert!(EngineTime::try_from(f64::NEG_INFINITY).is_err());

        let before = EngineTime::try_from(f64::MAX).unwrap();
        let after = before + Duration::from_millis(1);
        assert_eq!(after.as_secs_f64(), f64::MAX);

        let before = EngineTime::try_from(f64::MIN).unwrap();
        let after = before - Duration::from_millis(1);
        assert_eq!(after.as_secs_f64(), f64::MIN);
    }

    #[test]
    fn math() {
        let mut time = EngineTime::try_from(3.2).unwrap();
        time += Duration::from_secs_f64(4.1);

        assert_eq!(time.as_secs_f64(), 7.3);
    }
}
