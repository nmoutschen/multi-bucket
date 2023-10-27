use std::time::Duration;

pub(crate) struct Rate {
    pub(crate) refill: u64,
    pub(crate) interval: Duration,
    pub(crate) max: u64,
}

impl Rate {
    pub(crate) fn new(refill: u64, interval: Duration, max: u64) -> Self {
        Self {
            refill,
            interval,
            max,
        }
    }
}
