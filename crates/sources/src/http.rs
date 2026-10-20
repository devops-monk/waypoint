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

    /// Check the URL against the allowlist and wait for the host's slot.
    async fn checked(&self, url: &str, allowed_hosts: &[&str]) -> Result<Url, SourceError> {
        let parsed = Url::parse(url).map_err(|_| SourceError::BadUrl(url.into()))?;
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if parsed.scheme() != "https" || !allowed_hosts.iter().any(|pattern| host_matches(pattern, &host)) {
            return Err(SourceError::UntrustedHost(host));
        }
        self.throttle(&host).await;
        Ok(parsed)
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response, SourceError> {
        let resp = req.send().await?;
        let status = resp.status();
        if status.is_redirection() {
            return Err(SourceError::Status(status.as_u16(), "redirect refused".into()));
        }
        if !status.is_success() {
            return Err(SourceError::Status(status.as_u16(), status.canonical_reason().unwrap_or("").into()));
        }
        Ok(resp)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, url: &str, allowed_hosts: &[&str]) -> Result<T, SourceError> {
        self.get_json_with(url, allowed_hosts, &[]).await
    }

    /// GET with extra headers, for APIs that want a client key or CSRF marker.
    pub async fn get_json_with<T: DeserializeOwned>(&self, url: &str, allowed_hosts: &[&str], headers: &[(&str, &str)]) -> Result<T, SourceError> {
        let url = self.checked(url, allowed_hosts).await?;
        let mut req = self.client.get(url).header("Accept", "application/json");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        Ok(self.send(req).await?.json::<T>().await?)
    }

    /// GET a text body (RSS and XML feeds).
    pub async fn get_text(&self, url: &str, allowed_hosts: &[&str]) -> Result<String, SourceError> {
        let url = self.checked(url, allowed_hosts).await?;
        let req = self.client.get(url).header("Accept", "application/xml, application/rss+xml, text/xml;q=0.9, */*;q=0.5");
        Ok(self.send(req).await?.text().await?)
    }

    /// POST a JSON body and read JSON back (Workday's job search).
    pub async fn post_json<T: DeserializeOwned>(&self, url: &str, allowed_hosts: &[&str], body: &serde_json::Value) -> Result<T, SourceError> {
        let url = self.checked(url, allowed_hosts).await?;
        let req = self.client.post(url).header("Accept", "application/json").json(body);
        Ok(self.send(req).await?.json::<T>().await?)
    }
}

/// `*` matches exactly one DNS label, so `*.recruitee.com` allows `acme.recruitee.com`
/// but not `recruitee.com.evil.test` or `a.b.recruitee.com`.
pub(crate) fn host_matches(pattern: &str, host: &str) -> bool {
    let (p, h): (Vec<&str>, Vec<&str>) = (pattern.split('.').collect(), host.split('.').collect());
    p.len() == h.len()
        && p.iter().zip(&h).all(|(p, h)| {
            if *p == "*" { !h.is_empty() && h.len() <= 63 && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !h.starts_with('-') } else { p == h }
        })
}

#[cfg(test)]
mod tests {
    use super::host_matches;

    #[test]
    fn wildcards_match_one_label_only() {
        assert!(host_matches("api.ashbyhq.com", "api.ashbyhq.com"));
        assert!(host_matches("*.recruitee.com", "bunq.recruitee.com"));
        assert!(host_matches("*.*.myworkdayjobs.com", "nvidia.wd5.myworkdayjobs.com"));
        assert!(!host_matches("*.recruitee.com", "recruitee.com"));
        assert!(!host_matches("*.recruitee.com", "a.b.recruitee.com"));
        assert!(!host_matches("*.recruitee.com", "recruitee.com.evil.test"));
        assert!(!host_matches("*.recruitee.com", "-x.recruitee.com"));
    }
}
