//! Playback abuse protection: per-client request and session rate limits,
//! per-client concurrent stream limits, and byte pacing of stream bodies.
//!
//! All limiters are in-memory token buckets keyed by client address (IPv6
//! clients are grouped by /64, the usual allocation for a single subscriber).
//! State is per process; deployments with several replicas should also limit
//! at the reverse proxy or CDN.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
    time::Duration,
};

use axum::body::Bytes;
use futures_util::Stream;
// Tokio's clock so pacing honors paused time in tests.
use tokio::time::Instant;

use crate::{config::ProtectionConfig, error::AppError};

/// Client grouping key used by every limiter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientKey(IpAddr);

impl ClientKey {
    /// Normalizes an address: IPv4-mapped IPv6 becomes IPv4 and other IPv6
    /// addresses are truncated to their /64 network.
    #[must_use]
    pub fn from_ip(ip: IpAddr) -> Self {
        match ip {
            IpAddr::V4(v4) => Self(IpAddr::V4(v4)),
            IpAddr::V6(v6) => {
                if let Some(v4) = v6.to_ipv4_mapped() {
                    return Self(IpAddr::V4(v4));
                }
                let network = u128::from(v6) & !((1_u128 << 64) - 1);
                Self(IpAddr::V6(Ipv6Addr::from(network)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl Bucket {
    const fn full(capacity: f64, now: Instant) -> Self {
        Self {
            tokens: capacity,
            updated: now,
        }
    }

    fn refill(&mut self, now: Instant, rate: f64, capacity: f64) {
        let elapsed = now.saturating_duration_since(self.updated).as_secs_f64();
        self.tokens = (self.tokens + elapsed * rate).min(capacity);
        self.updated = now;
    }

    /// Takes `amount` tokens if available, otherwise returns the wait until
    /// they would be.
    fn try_take(
        &mut self,
        now: Instant,
        rate: f64,
        capacity: f64,
        amount: f64,
    ) -> Result<(), Duration> {
        self.refill(now, rate, capacity);
        if self.tokens >= amount {
            self.tokens -= amount;
            Ok(())
        } else {
            Err(Duration::from_secs_f64((amount - self.tokens) / rate))
        }
    }

    /// Takes `amount` tokens unconditionally (the balance may go negative)
    /// and returns how long the caller must wait to stay within `rate`.
    fn reserve(&mut self, now: Instant, rate: f64, capacity: f64, amount: f64) -> Duration {
        self.refill(now, rate, capacity);
        self.tokens -= amount;
        if self.tokens >= 0.0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(-self.tokens / rate)
        }
    }

    fn is_full(&self, now: Instant, rate: f64, capacity: f64) -> bool {
        let elapsed = now.saturating_duration_since(self.updated).as_secs_f64();
        self.tokens + elapsed * rate >= capacity
    }
}

/// Byte class of a stream response, selecting its pacing settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetClass {
    Playlist,
    Segment,
    Source,
}

#[derive(Debug)]
pub struct Protection {
    config: ProtectionConfig,
    requests: Mutex<HashMap<ClientKey, Bucket>>,
    sessions: Mutex<HashMap<ClientKey, Bucket>>,
    active_streams: Mutex<HashMap<ClientKey, u32>>,
    global: Mutex<Option<Bucket>>,
}

impl Protection {
    #[must_use]
    pub fn new(config: ProtectionConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            requests: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            active_streams: Mutex::new(HashMap::new()),
            global: Mutex::new(None),
        })
    }

    /// Admits a request that starts a playback session (watch or embed page).
    pub fn admit_session(&self, client: Option<ClientKey>) -> Result<(), AppError> {
        let per_minute = self.config.sessions_per_minute;
        let Some(client) = client.filter(|_| per_minute > 0) else {
            return Ok(());
        };
        let capacity = f64::from(per_minute);
        take(
            &self.sessions,
            client,
            capacity / 60.0,
            capacity,
            self.config.maximum_tracked_clients,
        )
    }

    /// Admits a stream request and reserves one concurrent-stream slot that
    /// is released when the returned permit (moved into the body) drops.
    pub fn admit_stream(
        self: &Arc<Self>,
        client: Option<ClientKey>,
    ) -> Result<Option<StreamPermit>, AppError> {
        let Some(client) = client else {
            return Ok(None);
        };
        let rate = self.config.stream_requests_per_second;
        if rate > 0 {
            take(
                &self.requests,
                client,
                f64::from(rate),
                f64::from(self.config.stream_request_burst),
                self.config.maximum_tracked_clients,
            )?;
        }
        let limit = self.config.max_concurrent_streams_per_client;
        if limit == 0 {
            return Ok(None);
        }
        let mut active = lock(&self.active_streams);
        let count = active.entry(client).or_insert(0);
        if *count >= limit {
            return Err(AppError::TooManyRequests {
                retry_after_seconds: 1,
            });
        }
        *count += 1;
        Ok(Some(StreamPermit {
            protection: Arc::clone(self),
            client,
        }))
    }

    /// Per-response pacing `(bytes_per_second, burst_bytes)` for an asset.
    #[must_use]
    pub const fn pacing(&self, class: AssetClass) -> Option<(u64, u64)> {
        let (rate, burst) = match class {
            AssetClass::Playlist => (0, 0),
            AssetClass::Segment => (
                self.config.segment_bytes_per_second,
                self.config.segment_burst_bytes,
            ),
            AssetClass::Source => (
                self.config.source_bytes_per_second,
                self.config.source_burst_bytes,
            ),
        };
        if rate == 0 { None } else { Some((rate, burst)) }
    }

    /// Reserves `bytes` from the shared egress budget.
    fn reserve_global(&self, bytes: usize) -> Duration {
        let rate = self.config.global_bytes_per_second;
        if rate == 0 {
            return Duration::ZERO;
        }
        let rate = rate as f64;
        let now = Instant::now();
        let mut global = lock(&self.global);
        global
            .get_or_insert_with(|| Bucket::full(rate, now))
            .reserve(now, rate, rate, bytes as f64)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Limiter state stays consistent even if a holder panicked mid-update.
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn take(
    buckets: &Mutex<HashMap<ClientKey, Bucket>>,
    client: ClientKey,
    rate: f64,
    capacity: f64,
    maximum_clients: usize,
) -> Result<(), AppError> {
    let now = Instant::now();
    let mut buckets = lock(buckets);
    if !buckets.contains_key(&client) && buckets.len() >= maximum_clients {
        buckets.retain(|_, bucket| !bucket.is_full(now, rate, capacity));
        if buckets.len() >= maximum_clients {
            // Fail open rather than grow without bound under address churn.
            tracing::warn!(
                tracked = buckets.len(),
                "rate limiter client table is full; resetting"
            );
            buckets.clear();
        }
    }
    buckets
        .entry(client)
        .or_insert_with(|| Bucket::full(capacity, now))
        .try_take(now, rate, capacity, 1.0)
        .map_err(|wait| AppError::TooManyRequests {
            retry_after_seconds: wait.as_secs().saturating_add(1),
        })
}

/// Concurrent-stream slot; released on drop.
#[derive(Debug)]
pub struct StreamPermit {
    protection: Arc<Protection>,
    client: ClientKey,
}

impl Drop for StreamPermit {
    fn drop(&mut self) {
        let mut active = lock(&self.protection.active_streams);
        if let Some(count) = active.get_mut(&self.client) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                active.remove(&self.client);
            }
        }
    }
}

/// Stream adapter that paces chunks to a per-response rate (after an initial
/// burst) and to the shared global egress budget, and holds the client's
/// concurrent-stream permit for the lifetime of the body.
pub struct PacedStream<S> {
    inner: S,
    protection: Arc<Protection>,
    local: Option<(Bucket, f64, f64)>,
    pending: Option<Bytes>,
    sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    _permit: Option<StreamPermit>,
}

impl<S> PacedStream<S> {
    pub fn new(
        inner: S,
        protection: Arc<Protection>,
        pacing: Option<(u64, u64)>,
        permit: Option<StreamPermit>,
    ) -> Self {
        let local = pacing.map(|(rate, burst)| {
            let rate = rate as f64;
            // A burst smaller than one chunk would only add latency.
            let capacity = (burst as f64).max(rate / 4.0);
            (Bucket::full(capacity, Instant::now()), rate, capacity)
        });
        Self {
            inner,
            protection,
            local,
            pending: None,
            sleep: None,
            _permit: permit,
        }
    }

    fn reserve(&mut self, bytes: usize) -> Duration {
        let now = Instant::now();
        let local = self
            .local
            .as_mut()
            .map_or(Duration::ZERO, |(bucket, rate, capacity)| {
                bucket.reserve(now, *rate, *capacity, bytes as f64)
            });
        local.max(self.protection.reserve_global(bytes))
    }
}

impl<S> Stream for PacedStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        if let Some(sleep) = this.sleep.as_mut() {
            if sleep.as_mut().poll(context).is_pending() {
                return Poll::Pending;
            }
            this.sleep = None;
            if let Some(bytes) = this.pending.take() {
                return Poll::Ready(Some(Ok(bytes)));
            }
        }
        match Pin::new(&mut this.inner).poll_next(context) {
            Poll::Ready(Some(Ok(bytes))) => {
                let wait = this.reserve(bytes.len());
                if wait.is_zero() {
                    return Poll::Ready(Some(Ok(bytes)));
                }
                let mut sleep = Box::pin(tokio::time::sleep(wait));
                if sleep.as_mut().poll(context).is_ready() {
                    return Poll::Ready(Some(Ok(bytes)));
                }
                this.pending = Some(bytes);
                this.sleep = Some(sleep);
                Poll::Pending
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use futures_util::StreamExt;

    use super::*;

    fn config() -> ProtectionConfig {
        ProtectionConfig {
            stream_requests_per_second: 2,
            stream_request_burst: 3,
            sessions_per_minute: 2,
            max_concurrent_streams_per_client: 2,
            ..ProtectionConfig::default()
        }
    }

    fn client(last: u8) -> Option<ClientKey> {
        Some(ClientKey::from_ip(IpAddr::V4(Ipv4Addr::new(
            192, 0, 2, last,
        ))))
    }

    #[test]
    fn ipv6_clients_are_grouped_by_prefix() {
        let first: IpAddr = "2001:db8:1:2:aaaa::1".parse().expect("ip");
        let second: IpAddr = "2001:db8:1:2:bbbb::2".parse().expect("ip");
        let other: IpAddr = "2001:db8:1:3::1".parse().expect("ip");
        let mapped: IpAddr = "::ffff:192.0.2.7".parse().expect("ip");
        assert_eq!(ClientKey::from_ip(first), ClientKey::from_ip(second));
        assert_ne!(ClientKey::from_ip(first), ClientKey::from_ip(other));
        assert_eq!(ClientKey::from_ip(mapped), client(7).expect("client"));
    }

    #[test]
    fn stream_requests_are_limited_per_client() {
        let protection = Protection::new(ProtectionConfig {
            max_concurrent_streams_per_client: 0,
            ..config()
        });
        for _ in 0..3 {
            protection.admit_stream(client(1)).expect("within burst");
        }
        let error = protection
            .admit_stream(client(1))
            .expect_err("burst exhausted");
        assert!(matches!(error, AppError::TooManyRequests { .. }));
        protection
            .admit_stream(client(2))
            .expect("other clients are unaffected");
        protection
            .admit_stream(None)
            .expect("unknown clients are not limited");
    }

    #[test]
    fn concurrent_streams_are_released_on_drop() {
        let protection = Protection::new(ProtectionConfig {
            stream_requests_per_second: 0,
            ..config()
        });
        let first = protection.admit_stream(client(1)).expect("first");
        let _second = protection.admit_stream(client(1)).expect("second");
        assert!(protection.admit_stream(client(1)).is_err());
        drop(first);
        protection
            .admit_stream(client(1))
            .expect("slot released after drop");
    }

    #[test]
    fn sessions_are_limited_per_minute() {
        let protection = Protection::new(config());
        protection.admit_session(client(1)).expect("first");
        protection.admit_session(client(1)).expect("second");
        assert!(protection.admit_session(client(1)).is_err());
    }

    #[test]
    fn client_table_is_bounded() {
        let protection = Protection::new(ProtectionConfig {
            maximum_tracked_clients: 4,
            max_concurrent_streams_per_client: 0,
            ..config()
        });
        for last in 0..50 {
            protection.admit_stream(client(last)).expect("admitted");
        }
        assert!(lock(&protection.requests).len() <= 4);
    }

    #[tokio::test(start_paused = true)]
    async fn paced_stream_respects_rate_after_burst() {
        let protection = Protection::new(ProtectionConfig::default());
        let chunks = (0..4).map(|_| Ok(Bytes::from(vec![0_u8; 1000])));
        let mut stream = PacedStream::new(
            futures_util::stream::iter(chunks),
            protection,
            Some((1000, 1000)),
            None,
        );
        let started = tokio::time::Instant::now();
        let mut total = 0;
        while let Some(chunk) = stream.next().await {
            total += chunk.expect("chunk").len();
        }
        assert_eq!(total, 4000);
        // 1000 bytes of burst, then 3000 bytes at 1000 B/s.
        let elapsed = started.elapsed();
        assert!(elapsed >= Duration::from_millis(2900), "{elapsed:?}");
        assert!(elapsed <= Duration::from_millis(3200), "{elapsed:?}");
    }
}
