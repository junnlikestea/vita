use crate::error::Result;
use crate::error::VitaError;
use crate::DataSource;
use async_trait::async_trait;
use crobat::Crobat;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, warn};

#[derive(Default, Clone)]
pub struct SonarSearch {
    client: Client,
}

impl SonarSearch {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl DataSource for SonarSearch {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        let mut client = Crobat::new().await;
        let subdomains = client.get_subs(host.clone()).await?;

        if !subdomains.is_empty() {
            info!("Discovered {} results for: {}", &subdomains.len(), &host);
            tx.send(subdomains).await;
            return Ok(());
        }

        warn!("No results for: {}", &host);
        Err(VitaError::SourceError("SonarSearch".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = SonarSearch::default().run(host, tx).await.unwrap();
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
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = SonarSearch::default().run(host, tx).await;
        let e = results.unwrap_err();
        assert_eq!(
            e.to_string(),
            "SonarSearch couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
