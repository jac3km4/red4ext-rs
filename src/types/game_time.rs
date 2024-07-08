use std::hash::Hash;

use crate::raw::root::RED4ext as red;
use crate::repr::{FromRepr, IntoRepr};

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct GameTime(red::GameTime);

impl GameTime {
    pub fn new(days: u32, hours: u32, minutes: u32, seconds: u32) -> Self {
        let mut this = Self::default();
        this.add_days(days);
        this.add_hours(hours);
        this.add_minutes(minutes);
        this.add_seconds(seconds);
        this
    }

    pub fn add_days(&mut self, days: u32) {
        self.0.seconds = self.0.seconds.saturating_add(
            days.saturating_mul(24)
                .saturating_mul(60)
                .saturating_mul(60),
        );
    }

    pub fn add_hours(&mut self, hours: u32) {
        self.0.seconds = self
            .0
            .seconds
            .saturating_add(hours.saturating_mul(60).saturating_mul(60));
    }

    pub fn add_minutes(&mut self, minutes: u32) {
        self.0.seconds = self.0.seconds.saturating_add(minutes.saturating_mul(60));
    }

    pub fn add_seconds(&mut self, seconds: u32) {
        self.0.seconds = self.0.seconds.saturating_add(seconds);
    }

    pub fn sub_days(&mut self, days: u32) {
        self.0.seconds = self.0.seconds.saturating_sub(
            days.saturating_mul(24)
                .saturating_mul(60)
                .saturating_mul(60),
        );
    }

    pub fn sub_hours(&mut self, hours: u32) {
        self.0.seconds = self
            .0
            .seconds
            .saturating_sub(hours.saturating_mul(60).saturating_mul(60));
    }

    pub fn sub_minutes(&mut self, minutes: u32) {
        self.0.seconds = self.0.seconds.saturating_sub(minutes.saturating_mul(60));
    }

    pub fn sub_seconds(&mut self, seconds: u32) {
        self.0.seconds = self.0.seconds.saturating_sub(seconds);
    }

    pub fn day(&self) -> u32 {
        unsafe { self.0.GetDay() }
    }

    pub fn hour(&self) -> u32 {
        unsafe { self.0.GetHour() }
    }

    pub fn minute(&self) -> u32 {
        unsafe { self.0.GetMinute() }
    }

    pub fn second(&self) -> u32 {
        unsafe { self.0.GetSecond() }
    }
}

impl std::fmt::Display for GameTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let [day, hour, min, sec] = unsafe { self.0.ToString() };
        write!(f, "{day}T{hour}:{min}:{sec}")
    }
}

impl PartialEq for GameTime {
    fn eq(&self, other: &Self) -> bool {
        self.0.seconds.eq(&other.0.seconds)
    }
}

impl Eq for GameTime {}

impl PartialOrd for GameTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GameTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.seconds.cmp(&other.0.seconds)
    }
}

impl Hash for GameTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.seconds.hash(state);
    }
}

impl From<u32> for GameTime {
    fn from(seconds: u32) -> Self {
        Self(red::GameTime { seconds })
    }
}

impl From<GameTime> for u32 {
    fn from(value: GameTime) -> Self {
        value.0.seconds
    }
}

impl IntoRepr for GameTime {
    type Repr = u32;

    fn into_repr(self) -> Self::Repr {
        self.0.seconds
    }
}

impl FromRepr for GameTime {
    type Repr = u32;

    fn from_repr(repr: Self::Repr) -> Self {
        Self::from(repr)
    }
}

#[cfg(feature = "time")]
impl TryFrom<GameTime> for time::Time {
    type Error = time::error::ComponentRange;

    fn try_from(value: GameTime) -> Result<Self, Self::Error> {
        Self::from_hms(
            value.hour() as u8,
            value.minute() as u8,
            value.second() as u8,
        )
    }
}

#[cfg(feature = "time")]
impl From<time::Time> for GameTime {
    fn from(value: time::Time) -> Self {
        Self::new(
            0,
            value.hour() as u32,
            value.minute() as u32,
            value.second() as u32,
        )
    }
}

#[cfg(feature = "time")]
impl std::ops::Add<time::Time> for GameTime {
    type Output = Self;

    fn add(self, rhs: time::Time) -> Self::Output {
        use std::ops::AddAssign;
        let mut copy = self;
        copy.add_assign(rhs);
        copy
    }
}

#[cfg(feature = "time")]
impl std::ops::AddAssign<time::Time> for GameTime {
    fn add_assign(&mut self, rhs: time::Time) {
        self.add_hours(rhs.hour() as u32);
        self.add_minutes(rhs.minute() as u32);
        self.add_seconds(rhs.second() as u32);
    }
}

#[cfg(feature = "time")]
impl std::ops::Sub<time::Time> for GameTime {
    type Output = Self;

    fn sub(self, rhs: time::Time) -> Self::Output {
        use std::ops::SubAssign;
        let mut copy = self;
        copy.sub_assign(rhs);
        copy
    }
}

#[cfg(feature = "time")]
impl std::ops::SubAssign<time::Time> for GameTime {
    fn sub_assign(&mut self, rhs: time::Time) {
        self.sub_hours(rhs.hour() as u32);
        self.sub_minutes(rhs.minute() as u32);
        self.sub_seconds(rhs.second() as u32);
    }
}

#[cfg(feature = "chrono")]
pub fn cyberpunk_epoch() -> chrono::DateTime<chrono_tz::Tz> {
    use chrono::TimeZone;
    chrono_tz::US::Pacific
        .with_ymd_and_hms(2077, 4, 16, 1, 24, 0)
        .unwrap()
}

#[cfg(feature = "chrono")]
impl From<GameTime> for chrono::DateTime<chrono::Utc> {
    fn from(value: GameTime) -> Self {
        // seconds being u32 it fits in i64, and nanos are zero
        Self::from_timestamp(
            cyberpunk_epoch().to_utc().timestamp() + value.0.seconds as i64,
            0,
        )
        .unwrap()
    }
}

#[cfg(feature = "chrono")]
impl TryFrom<chrono::DateTime<chrono::Utc>> for GameTime {
    type Error = chrono::format::ParseErrorKind;

    fn try_from(value: chrono::DateTime<chrono::Utc>) -> Result<Self, Self::Error> {
        if value < cyberpunk_epoch() {
            return Err(chrono::format::ParseErrorKind::OutOfRange);
        }
        Ok(Self::from(
            (value.timestamp() - cyberpunk_epoch().timestamp()) as u32,
        ))
    }
}

#[cfg(feature = "chrono")]
impl chrono::Timelike for GameTime {
    fn hour(&self) -> u32 {
        Self::hour(self)
    }

    fn minute(&self) -> u32 {
        Self::minute(self)
    }

    fn second(&self) -> u32 {
        Self::second(self)
    }

    fn nanosecond(&self) -> u32 {
        0
    }

    fn with_hour(&self, hour: u32) -> Option<Self> {
        if (0..=23).contains(&hour) {
            let mut copy = *self;
            unsafe { copy.0.SetHour(hour) };
            return Some(copy);
        }
        None
    }

    fn with_minute(&self, min: u32) -> Option<Self> {
        if (0..=59).contains(&min) {
            let mut copy = *self;
            unsafe { copy.0.SetMinute(min) };
            return Some(copy);
        }
        None
    }

    fn with_second(&self, sec: u32) -> Option<Self> {
        if (0..=59).contains(&sec) {
            let mut copy = *self;
            unsafe { copy.0.SetSecond(sec) };
            return Some(copy);
        }
        None
    }

    /// GameTime does not support nanoseconds, so it will always return `None`
    fn with_nanosecond(&self, _: u32) -> Option<Self> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::GameTime;

    #[test]
    fn instantiation() {
        let time = GameTime::new(2, 0, 7, 7);
        assert_eq!(time.day(), 2);
        assert_eq!(time.hour(), 0);
        assert_eq!(time.minute(), 7);
        assert_eq!(time.second(), 7);

        #[cfg(feature = "chrono")]
        {
            use chrono::Timelike;
            let time = time.with_minute(2).unwrap().with_second(2).unwrap();
            assert_eq!(time.day(), 2);
            assert_eq!(time.hour(), 0);
            assert_eq!(time.minute(), 2);
            assert_eq!(time.second(), 2);
        }
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn date() {
        use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

        use super::cyberpunk_epoch;
        let gt = GameTime::new(2, 0, 7, 7);
        let dt = DateTime::<Utc>::from(gt);
        assert_eq!(dt.year(), 2077);
        assert_eq!(dt.month(), 4);
        assert_eq!(dt.day(), 18);
        assert_eq!(dt.hour(), 8);
        assert_eq!(dt.minute(), 31);
        assert_eq!(dt.second(), 7);

        let dt = cyberpunk_epoch().to_utc()
            + Duration::days(2)
            + Duration::minutes(7)
            + Duration::seconds(7);
        let gt = GameTime::try_from(dt).unwrap();
        assert_eq!(gt.day(), 2);
        assert_eq!(gt.hour(), 0);
        assert_eq!(gt.minute(), 7);
        assert_eq!(gt.second(), 7);
    }

    #[test]
    #[cfg(feature = "time")]
    fn math() {
        let mut base = GameTime::new(2, 0, 7, 7);
        base += time::Time::from_hms(1, 2, 3).unwrap();

        assert_eq!(base.day(), 2);
        assert_eq!(base.hour(), 1);
        assert_eq!(base.minute(), 9);
        assert_eq!(base.second(), 10);

        let mut base = GameTime::new(2, 0, 7, 7);
        base += time::Time::from_hms(23, 53, 59).unwrap();

        assert_eq!(base.day(), 3);
        assert_eq!(base.hour(), 0);
        assert_eq!(base.minute(), 1);
        assert_eq!(base.second(), 6);
    }
}
