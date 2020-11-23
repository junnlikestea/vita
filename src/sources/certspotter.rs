use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

#[derive(Debug, Deserialize)]
struct CertSpotterResult {
    dns_names: Vec<String>,
}

impl IntoSubdomain for Vec<CertSpotterResult> {
    fn subdomains(&self) -> Vec<String> {
        self.iter().flat_map(|d| d.dns_names.to_owned()).collect()
    }
}

#[derive(Default, Clone)]
pub struct CertSpotter {
    client: Client,
}

impl CertSpotter {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!(
            "https://api.certspotter.com/v1/issuances?domain={}\
        &include_subdomains=true&expand=dns_names",
            host
        )
    }
}

#[async_trait]
impl DataSource for CertSpotter {
    async fn run(&self, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from certspotter for: {}", &host);
        let uri = self.build_url(&host);
        let resp: Option<Vec<CertSpotterResult>> =
            self.client.get(&uri).send().await?.json().await?;

        if let Some(data) = resp {
            let subdomains = data.subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                sender.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("No results for: {}", &host);
        Err(VitaError::SourceError("CertSpotter".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.certspotter.com/v1/issuances?domain=hackerone.com\
        &include_subdomains=true&expand=dns_names";
        assert_eq!(
            correct_uri,
            CertSpotter::default().build_url("hackerone.com")
        );
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = CertSpotter::default().run(host, tx).await;
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
        let res = CertSpotter::default().run(host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "CertSpotter couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
