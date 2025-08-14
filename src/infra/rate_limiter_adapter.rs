use crate::app::ports::RateLimiterPort;
use async_trait::async_trait;

pub struct RateLimiterAdapter(pub crate::rate_limiter::RateLimiter);

#[async_trait]
impl RateLimiterPort for RateLimiterAdapter {
    async fn acquire(&self, bytes: u64) {
        self.0.acquire(bytes).await;
    }
}

