use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

#[derive(Deserialize, Debug)]
struct Subdomain {
    hostname: String,
}

#[derive(Deserialize, Debug)]
struct AlienvaultResult {
    passive_dns: Vec<Subdomain>,
    count: i32,
}

impl IntoSubdomain for AlienvaultResult {
    fn subdomains(&self) -> Vec<String> {
        self.passive_dns
            .iter()
            .map(|s| s.hostname.to_owned())
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct AlienVault {
    client: Client,
}

impl AlienVault {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!(
            "https://otx.alienvault.com/api/v1/indicators/domain/{}/passive_dns",
            host
        )
    }
}

#[async_trait]
impl DataSource for AlienVault {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from alienvault for: {}", &host);
        let uri = self.build_url(&host);
        let resp: AlienvaultResult = self.client.get(&uri).send().await?.json().await?;

        if resp.count != 0 {
            let subdomains = resp.subdomains();
            info!("Discovered {} results for {}", &subdomains.len(), &host);
            let _ = tx.send(subdomains).await;
            return Ok(());
        }

        warn!("No results for {} from AlienVault", &host);
        Err(VitaError::SourceError("AlienVault".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://otx.alienvault.com/api/v1/indicators/domain/\
        hackerone.com/passive_dns";

        assert_eq!(
            correct_uri,
            AlienVault::default().build_url("hackerone.com")
        );
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_string());
        let _ = AlienVault::default().run(host, tx).await.unwrap();
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
        assert!(matches!(
            AlienVault::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ))
    }
}
