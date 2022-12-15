use std::time::{Duration, Instant};

/// Higher-precision sleep for very short durations. thread::sleep relies on OS scheduling, which
/// is often too slow.
pub fn sleep_precise(dur: Duration) {
    // This function itself adds a bit of delay, so we subtract a bit to account for that.
    let dur = dur.checked_sub(Duration::from_nanos(100)).unwrap_or(dur);
    let start = Instant::now();
    loop {
        if start.elapsed() >= dur {
            return;
        }
    }
}
