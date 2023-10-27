use bucket::Bucket;
use hashlink::LinkedHashMap;
use rate::Rate;
use std::{
    hash::Hash,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

mod bucket;
mod rate;

pub struct Buckets<K> {
    buckets: Mutex<LinkedHashMap<K, Arc<Bucket>>>,

    rate: Rate,
    buffer: Duration,
    start: Instant,
}

impl<K> Buckets<K>
where
    K: Eq + Hash + Clone,
{
    pub fn new(refill: u64, interval: Duration, max: u64, buffer: Duration) -> Self {
        Self {
            buckets: Mutex::new(LinkedHashMap::new()),

            rate: Rate::new(refill, interval, max),
            buffer,
            start: Instant::now(),
        }
    }

    pub fn try_acquire(&self, key: K, qty: u64) -> Result<u64, Error> {
        let elapsed = self.start.elapsed();
        let bucket: Arc<Bucket> = self.get(key, elapsed);
        bucket.refill(elapsed, &self.rate);

        bucket.try_acquire(qty)
    }

    pub fn available(&self, key: K) -> u64 {
        let elapsed = self.start.elapsed();
        let bucket: Arc<Bucket> = self.get(key, elapsed);
        bucket.available()
    }

    pub fn len(&self) -> usize {
        // TODO: handle lock poisoning cleanly
        self.buckets.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        // TODO: handle lock poisoning cleanly
        self.buckets.lock().unwrap().is_empty()
    }

    /// Get or create a new [`Bucket`]
    fn get(&self, key: K, elapsed: Duration) -> Arc<Bucket> {
        // TODO: handle lock poisoning cleanly
        let mut buckets = self.buckets.lock().unwrap();
        self.remove_expired(elapsed, &mut *buckets);
        buckets
            .entry(key)
            .or_insert_with(|| Arc::new(Bucket::new(self.rate.max, elapsed + self.rate.interval)))
            .clone()
    }

    /// Remove all old [`Bucket`]s
    fn remove_expired(&self, elapsed: Duration, buckets: &mut LinkedHashMap<K, Arc<Bucket>>) {
        let mut expired_keys = Vec::new();

        for (key, bucket) in buckets.iter() {
            if bucket.full_at(&self.rate) + self.buffer >= elapsed {
                break;
            }

            expired_keys.push(key.clone());
        }

        for k in expired_keys {
            buckets.remove(&k);
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// High contention to update the number of available tokens
    #[error("high contention to update available token")]
    HighContention,

    /// Not enough tokens available
    #[error("not enough tokens available")]
    NotEnoughTokens,
}
