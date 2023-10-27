use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{rate::Rate, Error};

pub(crate) struct Bucket {
    pub(crate) available: AtomicU64,
    refill_at: AtomicU64,
}

impl Bucket {
    pub(crate) fn new(available: u64, refill_at: Duration) -> Self {
        Self {
            available: AtomicU64::new(available),
            refill_at: AtomicU64::new(refill_at.as_millis() as u64),
        }
    }

    pub(crate) fn available(&self) -> u64 {
        self.available.load(Ordering::Acquire)
    }

    pub(crate) fn full_at(&self, rate: &Rate) -> Duration {
        let available = self.available.load(Ordering::Acquire);
        let refill_at = Duration::from_millis(self.refill_at.load(Ordering::Acquire));
        let intervals = rate.interval * (rate.max - available).div_ceil(rate.refill) as u32;

        refill_at + intervals
    }

    pub(crate) fn refill(&self, elapsed: Duration, rate: &Rate) {
        let mut intervals;

        loop {
            let refill_at = Duration::from_millis(self.refill_at.load(Ordering::Relaxed));

            // Next refill is not due yet, return early
            if elapsed < refill_at {
                return;
            }

            // Number of intervals
            // 1 for the time until `refill_at`, then 1 for every time intervals since then
            intervals = (1 + (elapsed - refill_at).as_nanos() / rate.interval.as_nanos()) as u64;

            // Update the `refill_at` time
            let next_refill_at = refill_at + (rate.interval * intervals as u32);
            if self
                .refill_at
                .compare_exchange(
                    refill_at.as_millis() as u64,
                    next_refill_at.as_millis() as u64,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                break;
            }
        }

        let amount = intervals * rate.refill;
        let available = self.available.load(Ordering::Acquire);

        if available + amount >= rate.max {
            self.available
                .fetch_add(rate.max - available, Ordering::Release);
        } else {
            self.available.fetch_add(amount, Ordering::Release);
        }
    }

    /// Try to acquire `num` tokens
    pub fn try_acquire(&self, num: u64) -> Result<u64, Error> {
        // Compare-and-swap loop
        //
        // If there aren't enough tokens available, this will break early.
        // If there are enough tokens, but the number of available tokens is updated before this
        // call can, it will loop until it can, or it tried 65536 times.
        for _ in 0..0x10000 {
            let available = self.available.load(Ordering::Acquire);
            if available < num {
                return Err(Error::NotEnoughTokens);
            }

            let new = available.saturating_sub(num);

            if self
                .available
                .compare_exchange(available, new, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(num);
            }
        }

        // Could not update the number of available tokens after 65536 attempts. Contention is too,
        // return an error instead of trying ad infinitum.
        Err(Error::HighContention)
    }
}
