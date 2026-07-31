/// TimeFrame 枚举：表示 K 线的时间周期
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFrame {
    Ticks,
    Seconds,
    Minutes,
    Days,
    Weeks,
    Months,
    Years,
}

impl std::fmt::Display for TimeFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeFrame::Ticks => write!(f, "Ticks"),
            TimeFrame::Seconds => write!(f, "Seconds"),
            TimeFrame::Minutes => write!(f, "Minutes"),
            TimeFrame::Days => write!(f, "Days"),
            TimeFrame::Weeks => write!(f, "Weeks"),
            TimeFrame::Months => write!(f, "Months"),
            TimeFrame::Years => write!(f, "Years"),
        }
    }
}
