use std::num::NonZeroU64;
use std::time::Duration;

/// Engine-clock ticks.
/// Zero is invalid. Convert using the clock's `QueryPerformanceFrequency()`
/// value in ticks per second; no fixed frequency is assumed. Conversions and
/// arithmetic accept zero ticks but do not make that value valid.
///
/// SDK: [EngineTime](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Natives/EngineTime.hpp).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct EngineTime(u64);

impl EngineTime {
    /// Returns whether the tick count is nonzero.
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// Wraps a raw tick count, including the invalid zero value.
    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Returns the raw tick count.
    pub const fn ticks(self) -> u64 {
        self.0
    }

    /// Converts to seconds, potentially losing precision for large tick counts.
    pub fn as_secs_f64(self, frequency: NonZeroU64) -> f64 {
        self.0 as f64 / frequency.get() as f64
    }

    /// Converts nonnegative, finite seconds, truncating fractional ticks.
    ///
    /// # Errors
    /// Returns `EngineTimeError::NotANumber` for NaN, or
    /// `EngineTimeError::OutOfBounds` for negative, infinite, or overflowing values.
    pub fn from_secs_f64(seconds: f64, frequency: NonZeroU64) -> Result<Self, EngineTimeError> {
        if seconds.is_nan() {
            return Err(EngineTimeError::NotANumber);
        }
        let ticks = seconds * frequency.get() as f64;
        // u64::MAX rounds up to 2^64 in f64, which must be rejected.
        if seconds < 0.0 || !ticks.is_finite() || ticks >= 18_446_744_073_709_551_616.0 {
            return Err(EngineTimeError::OutOfBounds);
        }
        Ok(Self(ticks as u64))
    }

    /// Converts a duration using integer arithmetic, truncating fractional ticks.
    ///
    /// # Errors
    /// Returns `EngineTimeError::OutOfBounds` if the tick count exceeds `u64::MAX`.
    pub fn from_duration(
        duration: Duration,
        frequency: NonZeroU64,
    ) -> Result<Self, EngineTimeError> {
        let frequency = u128::from(frequency.get());
        let ticks = u128::from(duration.as_secs()) * frequency
            + u128::from(duration.subsec_nanos()) * frequency / 1_000_000_000;
        u64::try_from(ticks)
            .map(Self)
            .map_err(|_| EngineTimeError::OutOfBounds)
    }

    /// Converts to a duration, truncating fractional nanoseconds.
    pub fn to_duration(self, frequency: NonZeroU64) -> Duration {
        let seconds = self.0 / frequency.get();
        let nanos =
            u128::from(self.0 % frequency.get()) * 1_000_000_000 / u128::from(frequency.get());
        Duration::new(seconds, nanos as u32)
    }

    /// Adds a duration after truncating it to whole ticks.
    /// Returns `None` if duration conversion or addition overflows.
    pub fn checked_add_duration(self, duration: Duration, frequency: NonZeroU64) -> Option<Self> {
        self.0
            .checked_add(Self::from_duration(duration, frequency).ok()?.0)
            .map(Self)
    }

    /// Subtracts a duration after truncating it to whole ticks.
    /// Returns `None` if duration conversion overflows or subtraction underflows.
    pub fn checked_sub_duration(self, duration: Duration, frequency: NonZeroU64) -> Option<Self> {
        self.0
            .checked_sub(Self::from_duration(duration, frequency).ok()?.0)
            .map(Self)
    }
}

/// Failure to convert seconds or a duration into engine-clock ticks.
#[derive(Debug)]
pub enum EngineTimeError {
    /// The input is negative, infinite, or too large for an unsigned 64-bit tick count.
    OutOfBounds,
    /// The floating-point seconds value is NaN.
    NotANumber,
}

impl std::fmt::Display for EngineTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OutOfBounds => "engine time must fit in unsigned 64-bit ticks",
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
    fn layout_and_ticks() {
        assert_eq!(std::mem::size_of::<EngineTime>(), 8);
        assert_eq!(
            std::mem::align_of::<EngineTime>(),
            std::mem::align_of::<u64>()
        );
        assert!(!EngineTime::default().is_valid());
        assert_eq!(EngineTime::from_ticks(u64::MAX).ticks(), u64::MAX);
    }

    #[test]
    fn frequency_aware_conversions() {
        let frequency = std::num::NonZeroU64::new(10_000_000).unwrap();
        let time = EngineTime::from_duration(Duration::from_millis(3200), frequency).unwrap();
        assert_eq!(time.ticks(), 32_000_000);
        assert_eq!(time.as_secs_f64(frequency), 3.2);
        assert_eq!(time.to_duration(frequency), Duration::from_millis(3200));
        let after = time
            .checked_add_duration(Duration::from_millis(4100), frequency)
            .unwrap();
        assert_eq!(after.ticks(), 73_000_000);
        assert_eq!(
            after.checked_sub_duration(Duration::from_millis(4100), frequency),
            Some(time)
        );
        let thirds = std::num::NonZeroU64::new(3).unwrap();
        assert_eq!(EngineTime::from_secs_f64(0.5, thirds).unwrap().ticks(), 1);
        assert_eq!(
            EngineTime::from_ticks(1).to_duration(thirds),
            Duration::from_nanos(333_333_333)
        );
    }

    #[test]
    fn bounds() {
        let frequency = std::num::NonZeroU64::new(1).unwrap();
        for seconds in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            18_446_744_073_709_551_616.0,
        ] {
            assert!(EngineTime::from_secs_f64(seconds, frequency).is_err());
        }
        assert_eq!(
            EngineTime::from_duration(Duration::from_secs(u64::MAX), frequency)
                .unwrap()
                .ticks(),
            u64::MAX
        );
        assert!(
            EngineTime::from_duration(Duration::MAX, std::num::NonZeroU64::new(u64::MAX).unwrap())
                .is_err()
        );
        assert!(
            EngineTime::from_ticks(u64::MAX)
                .checked_add_duration(Duration::from_secs(1), frequency)
                .is_none()
        );
        assert!(
            EngineTime::default()
                .checked_sub_duration(Duration::from_secs(1), frequency)
                .is_none()
        );
    }
}
