use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

#[derive(Deserialize)]
struct Subdomain {
    id: String,
}

#[derive(Deserialize)]
struct VirustotalResult {
    data: Option<Vec<Subdomain>>,
}

impl IntoSubdomain for VirustotalResult {
    fn subdomains(&self) -> Vec<String> {
        self.data
            .iter()
            .flatten()
            .map(|s| s.id.to_owned())
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct VirusTotal {
    client: Client,
}

impl VirusTotal {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        // TODO: can we gather the subdomains using:
        // Handle pagenation
        // https://www.virustotal.com/vtapi/v2/domain/report
        format!(
            "https://www.virustotal.com/ui/domains/{}/subdomains?limit=40",
            host
        )
    }
}

#[async_trait]
impl DataSource for VirusTotal {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from virustotal for: {}", &host);
        let uri = self.build_url(&host);
        let resp: VirustotalResult = self.client.get(&uri).send().await?.json().await?;

        let subdomains = resp.subdomains();
        if !subdomains.is_empty() {
            info!("Discovered {} results for {}", &subdomains.len(), &host);
            let _ = tx.send(subdomains).await;
            return Ok(());
        }

        warn!("no results found for {} from VirusTotal", &host);
        Err(VitaError::SourceError("VirusTotal".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    // IGNORE by default since we have limited api calls.
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = VirusTotal::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            VirusTotal::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
