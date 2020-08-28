use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
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

fn build_url(host: &str) -> String {
    format!(
        "https://www.threatcrowd.org/searchApi/v2/domain/report/?domain={}",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from threatcrowd for: {}", &host);
    let uri = build_url(&host);
    let resp: ThreatCrowdResult = client.get(&uri).send().await?.json().await?;
    let subdomains = resp.subdomains();

    debug!("threatcrowd response: {:?}", &resp);
    if !subdomains.is_empty() {
        info!("Discovered {} results for {}", &subdomains.len(), &host);
        Ok(subdomains)
    } else {
        warn!("No results found for: {}", &host);
        Err(Error::source_error("ThreatCrowd", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "ThreatCrowd couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
