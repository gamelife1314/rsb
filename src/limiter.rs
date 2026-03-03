use std::num::NonZeroU32;

use anyhow::anyhow;
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::{InMemoryState, direct::NotKeyed},
};
use tokio::time;

/// Limiter limit only sending a fixed number of requests per second
pub(crate) struct Limiter {
    inner: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
}

impl Limiter {
    /// create a new Limiter
    pub(crate) fn new(rate: u16) -> Limiter {
        Self {
            inner: RateLimiter::direct(Quota::per_second(
                NonZeroU32::new(rate as u32).unwrap(),
            )),
        }
    }

    /// each check returns quickly, may fail or succeed
    pub(crate) async fn allow_fast(&self) -> anyhow::Result<()> {
        self.inner
            .check()
            .map_err(|_| anyhow!("no available token"))
    }

    /// allow to obtain n tokens at one time
    pub(crate) fn allow_n(&self, n: usize) {
        loop {
            let result = self.inner.check_n(NonZeroU32::new(n as u32).unwrap());
            if result.is_ok() {
                break;
            }
            std::thread::sleep(time::Duration::from_nanos(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_new() {
        let limiter = Limiter::new(10);
        // Just test that it creates without panicking
        let _ = limiter;
    }

    #[test]
    fn test_limiter_allow_n() {
        let limiter = Limiter::new(10);
        limiter.allow_n(5);
        // Test that it completes without panicking
    }

    #[tokio::test]
    async fn test_limiter_allow_fast() {
        let limiter = Limiter::new(10);
        // First request should succeed
        let result = limiter.allow_fast().await;
        assert!(result.is_ok());

        // Try to consume all tokens
        for _ in 0..9 {
            let _ = limiter.allow_fast().await;
        }

        // Next request should fail
        let result = limiter.allow_fast().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_limiter_rate_limiting() {
        let limiter = Limiter::new(2);
        // First two requests should succeed
        assert!(limiter.allow_fast().await.is_ok());
        assert!(limiter.allow_fast().await.is_ok());

        // Third request should fail
        assert!(limiter.allow_fast().await.is_err());
    }
}
