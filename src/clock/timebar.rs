use std::fmt::Display;

use chrono::{Duration, TimeDelta};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeBarLength {
    Minute,
    Hour,
    Custom(i64),
    /// implementing a bar that would grow smaller would be weird, so it's a count up instead of
    /// a countdown
    Countup(i64),
    Day,
}

impl TimeBarLength {
    pub(crate) const fn as_secs(self) -> i64 {
        match self {
            Self::Minute => 60,
            Self::Day => 24 * 60 * 60,
            Self::Hour => 60 * 60,
            Self::Custom(secs) | Self::Countup(secs) => secs,
        }
    }
}

impl Default for TimeBarLength {
    fn default() -> Self {
        Self::Minute
    }
}

impl Display for TimeBarLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buf = match self {
            Self::Minute => humantime::Duration::from(
                Duration::minutes(1)
                    .to_std()
                    .expect("could not convert chrono time to std time"),
            ),
            Self::Hour => humantime::Duration::from(
                Duration::hours(1)
                    .to_std()
                    .expect("could not convert chrono time to std time"),
            ),
            Self::Day => humantime::Duration::from(
                Duration::days(1)
                    .to_std()
                    .expect("could not convert chrono time to std time"),
            ),
            Self::Custom(secs) | Self::Countup(secs) => humantime::Duration::from(
                Duration::seconds(*secs)
                    .to_std()
                    .expect("could not convert chrono time to std time"),
            ),
        };
        write!(f, "{buf}")
    }
}
