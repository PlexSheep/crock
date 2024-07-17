#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeBarLength {
    Minute,
    Hour,
    Custom(i128),
    /// implementing a bar that would grow smaller would be weird, so it's a count up instead of
    /// a countdown
    Countup(i128),
    Day,
}

impl TimeBarLength {
    pub(crate) const fn as_secs(self) -> i128 {
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
