use crate::IntoSubdomain;
use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::{error::Error, fmt};

#[derive(Deserialize)]
struct ThreatCrowdResult {
    subdomains: Option<Vec<String>>,
}

impl IntoSubdomain for ThreatCrowdResult {
    fn subdomains(&self) -> HashSet<String> {
        self.subdomains
            .iter()
            .flatten()
            .map(|s| s.to_string())
            .collect()
    }
}

#[derive(Debug)]
struct ThreatCrowdError {
    host: Arc<String>,
}

impl ThreatCrowdError {
    fn new(host: Arc<String>) -> Self {
        Self { host: host }
    }
}

impl Error for ThreatCrowdError {}

impl fmt::Display for ThreatCrowdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ThreatCrowd couldn't find any results for: {}",
            self.host
        )
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://www.threatcrowd.org/searchApi/v2/domain/report/?domain={}",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let resp: ThreatCrowdResult = surf::get(uri).recv_json().await?;
    let subdomains = resp.subdomains();

    if subdomains.len() != 0 {
        return Ok(subdomains);
    } else {
        Err(Box::new(ThreatCrowdError::new(host)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 5);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "ThreatCrowd couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
