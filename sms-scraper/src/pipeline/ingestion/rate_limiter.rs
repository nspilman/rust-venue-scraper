use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

#[derive(Clone, Debug, Default)]
pub struct Limits {
    pub requests_per_min: Option<u64>,
    pub bytes_per_min: Option<u64>,
    pub concurrency: Option<u32>,
}

#[derive(Debug)]
pub struct RateLimiter {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    limits: Limits,
    // token buckets are modeled by the time of last refill and the current tokens
    rpm_tokens: Mutex<(f64, Instant)>,
    bpm_tokens: Mutex<(f64, Instant)>,
    sem: Option<Semaphore>,
}

impl RateLimiter {
    pub fn new(limits: Limits) -> Self {
        let now = Instant::now();
        let rpm_capacity = limits.requests_per_min.unwrap_or(0) as f64;
        let bpm_capacity = limits.bytes_per_min.unwrap_or(0) as f64;
        let sem = limits.concurrency.map(|c| Semaphore::new(c as usize));
        Self {
            inner: Arc::new(Inner {
                limits,
                rpm_tokens: Mutex::new((rpm_capacity, now)),
                bpm_tokens: Mutex::new((bpm_capacity, now)),
                sem,
            }),
        }
    }

    // Acquire permission for a request of given size (bytes). Await as needed.
    pub async fn acquire(&self, bytes: u64) {
        // Concurrency first
        let _permit = if let Some(sem) = &self.inner.sem {
            Some(sem.acquire().await.expect("semaphore closed"))
        } else {
            None
        };

        // Requests per minute bucket
        if let Some(rpm) = self.inner.limits.requests_per_min {
            if rpm > 0 {
                self.consume_tokens(&self.inner.rpm_tokens, rpm as f64, 60.0, 1.0)
                    .await;
            }
        }
        // Bytes per minute bucket
        if let Some(bpm) = self.inner.limits.bytes_per_min {
            if bpm > 0 {
                self.consume_tokens(&self.inner.bpm_tokens, bpm as f64, 60.0, bytes as f64)
                    .await;
            }
        }
        // _permit dropped here when function returns, releasing concurrency
    }

    async fn consume_tokens(
        &self,
        bucket: &Mutex<(f64, Instant)>,
        capacity: f64,
        period_secs: f64,
        cost: f64,
    ) {
        // Basic token bucket: refill continuously, wait until enough tokens accumulate
        loop {
            let mut guard = bucket.lock().await;
            let (ref mut tokens, ref mut last) = *guard;
            let now = Instant::now();
            let elapsed = now.duration_since(*last).as_secs_f64();
            let refill_rate = capacity / period_secs; // tokens per second
            *tokens = (*tokens + elapsed * refill_rate).min(capacity);
            *last = now;
            if *tokens >= cost {
                *tokens -= cost;
                break;
            } else {
                // compute needed time to get enough tokens
                let need = cost - *tokens;
                let secs = need / refill_rate;
                drop(guard);
                tokio::time::sleep(Duration::from_secs_f64(secs.max(0.001))).await;
            }
        }
    }
}
