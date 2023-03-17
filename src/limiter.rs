use std::num::NonZeroU32;

use anyhow::anyhow;
use governor::{
    clock::DefaultClock,
    state::{direct::NotKeyed, InMemoryState},
    Quota, RateLimiter,
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
