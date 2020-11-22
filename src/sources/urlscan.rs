use crate::error::{Error, Result};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, trace, warn};

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
    results: Vec<UrlScanPage>,
}

impl IntoSubdomain for UrlScanResult {
    fn subdomains(&self) -> Vec<String> {
        self.results
            .iter()
            .map(|s| s.page.domain.to_owned())
            .collect()
    }
}

#[derive(Default)]
pub struct UrlScan {
    client: Client,
}

impl UrlScan {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!("https://urlscan.io/api/v1/search/?q=domain:{}", host)
    }
}

#[async_trait]
impl DataSource for UrlScan {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from urlscan for: {}", &host);
        let uri = self.build_url(&host);
        let resp: Option<UrlScanResult> = self.client.get(&uri).send().await?.json().await?;

        match resp {
            Some(d) => {
                let subdomains = d.subdomains();

                if !subdomains.is_empty() {
                    info!("Discovered {} results for: {}", &subdomains.len(), &host);
                    if let Err(e) = tx.send(subdomains).await {
                        error!("got error {} when sending to channel", e)
                    }
                    Ok(())
                } else {
                    warn!("No results found for: {}", &host);
                    Err(Error::source_error("UrlScan", host))
                }
            }

            None => Err(Error::source_error("UrlScan", host)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://urlscan.io/api/v1/search/?q=domain:hackerone.com";
        assert_eq!(correct_uri, UrlScan::default().build_url("hackerone.com"));
    }

    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = UrlScan::default().run(host, tx).await;
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
        let res = UrlScan::default().run(host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "UrlScan couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
