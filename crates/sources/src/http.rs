//! Shared HTTP client: HTTPS-only, host-allowlisted, no redirects, per-host throttled.
//! The allowlist + no-redirect pair is the SSRF guard career-ops providers use
//! (`assertGreenhouseUrl` + `redirect: 'error'`), applied once here for every source.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use url::Url;

use crate::SourceError;

#[derive(Clone)]
pub struct Http {
    client: reqwest::Client,
    min_interval: Duration,
    last_hit: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Http {
    pub fn new(min_interval: Duration) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(concat!("Waypoint/", env!("CARGO_PKG_VERSION"), " (personal job search; +https://github.com/)"))
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .https_only(true)
            .build()
            .expect("reqwest client");
        Http { client, min_interval, last_hit: Arc::default() }
    }

    /// Wait until this host's slot is free, so no host sees more than one request per interval.
    async fn throttle(&self, host: &str) {
        loop {
            let wait = {
                let mut map = self.last_hit.lock().await;
                let now = Instant::now();
                match map.get(host) {
                    Some(prev) if now.duration_since(*prev) < self.min_interval => self.min_interval - now.duration_since(*prev),
                    _ => {
                        map.insert(host.to_string(), now);
                        return;
                    }
                }
            };
            tokio::time::sleep(wait).await;
        }
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str, allowed_hosts: &[&str]) -> Result<T, SourceError> {
        let parsed = Url::parse(url).map_err(|_| SourceError::BadUrl(url.into()))?;
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if parsed.scheme() != "https" || !allowed_hosts.contains(&host.as_str()) {
            return Err(SourceError::UntrustedHost(host));
        }
        self.throttle(&host).await;
        let resp = self.client.get(parsed).header("Accept", "application/json").send().await?;
        let status = resp.status();
        if status.is_redirection() {
            return Err(SourceError::Status(status.as_u16(), "redirect refused".into()));
        }
        if !status.is_success() {
            return Err(SourceError::Status(status.as_u16(), status.canonical_reason().unwrap_or("").into()));
        }
        Ok(resp.json::<T>().await?)
    }
}
