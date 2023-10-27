use std::time::Duration;

use multi_bucket::Buckets;

/// Test if the [`Buckets`] will return the right number of available tokens
#[test]
fn simple() {
    let buckets = Buckets::new(1, Duration::from_secs(1), 100, Duration::from_secs(5));

    // Used bucket
    buckets.try_acquire(1, 30).unwrap();
    assert_eq!(buckets.available(1), 70);
    assert!(matches!(buckets.try_acquire(1, 90), Err(_)));

    // New bucket
    assert_eq!(buckets.available(2), 100);
}

/// Test if the [`Buckets`] will clean up stale entries
#[test]
fn cleanup() {
    let buckets = Buckets::new(100, Duration::from_millis(1), 100, Duration::ZERO);

    // Acquire create buckets
    buckets.try_acquire(1, 1).unwrap();
    buckets.try_acquire(2, 1).unwrap();
    // There are now 2 buckets
    assert_eq!(buckets.len(), 2);
    // Wait after the expiry of bucket 1
    std::thread::sleep(Duration::from_millis(2));
    // Acquire to force removing expired buckets
    buckets.try_acquire(2, 1).unwrap();
    // There should be only 1 bucket left
    assert_eq!(buckets.len(), 1);
}
