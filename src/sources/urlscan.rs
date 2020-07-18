use crate::error::{Error, Result};
use crate::IntoSubdomain;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize, Hash, Eq, PartialEq)]
struct UrlScanPage {
    page: UrlScanDomain,
}

#[derive(Deserialize, Eq, Hash, PartialEq)]
struct UrlScanDomain {
    domain: String,
}

#[derive(Deserialize)]
struct UrlScanResult {
    results: HashSet<UrlScanPage>,
}

impl IntoSubdomain for UrlScanResult {
    fn subdomains(&self) -> HashSet<String> {
        self.results
            .iter()
            .map(|s| s.page.domain.to_string())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://urlscan.io/api/v1/search/?q=domain:{}", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let resp: Option<UrlScanResult> = surf::get(uri).recv_json().await?;

    match resp {
        Some(d) => {
            let subdomains = d.subdomains();
            if !subdomains.is_empty() {
                Ok(subdomains)
            } else {
                Err(Error::source_error("UrlScan", host))
            }
        }

        None => Err(Error::source_error("UrlScan", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://urlscan.io/api/v1/search/?q=domain:hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "UrlScan couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
