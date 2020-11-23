use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

#[derive(Debug, Deserialize)]
struct ThreatCrowdResult {
    subdomains: Option<Vec<String>>,
}

impl IntoSubdomain for ThreatCrowdResult {
    fn subdomains(&self) -> Vec<String> {
        self.subdomains
            .iter()
            .flatten()
            .map(|s| s.to_owned())
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct ThreatCrowd {
    client: Client,
}

impl ThreatCrowd {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!(
            "https://www.threatcrowd.org/searchApi/v2/domain/report/?domain={}",
            host
        )
    }
}

#[async_trait]
impl DataSource for ThreatCrowd {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from threatcrowd for: {}", &host);
        let uri = self.build_url(&host);
        let resp: ThreatCrowdResult = self.client.get(&uri).send().await?.json().await?;
        let subdomains = resp.subdomains();

        if !subdomains.is_empty() {
            info!("Discovered {} results for {}", &subdomains.len(), &host);
            tx.send(subdomains).await;
            return Ok(());
        }

        warn!("No results found for: {} from Threatcrowd", &host);
        Err(VitaError::SourceError("ThreatCrowd".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = ThreatCrowd::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = ThreatCrowd::default().run(host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "ThreatCrowd couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
