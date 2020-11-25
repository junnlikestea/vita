use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::value::Value;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct AnubisResult {
    results: Value,
}

impl AnubisResult {
    fn new(results: Value) -> Self {
        Self { results }
    }
}

impl IntoSubdomain for AnubisResult {
    fn subdomains(&self) -> Vec<String> {
        match self.results.as_array() {
            Some(array) => array.iter().map(|s| s.to_string()).collect(),
            None => Vec::new(),
        }
    }
}

#[derive(Default, Clone)]
pub struct AnubisDB {
    client: Client,
}

impl AnubisDB {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!("https://jldc.me/anubis/subdomains/{}", host)
    }
}

#[async_trait]
impl DataSource for AnubisDB {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from anubisdb for: {}", &host);
        let uri = self.build_url(&host);
        let resp: Option<Value> = self.client.get(&uri).send().await?.json().await?;

        if let Some(data) = resp {
            let subdomains = AnubisResult::new(data).subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("No results for {} from AnubisDB", &host);
        Err(VitaError::SourceError("AnubisDB".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://jldc.me/anubis/subdomains/hackerone.com";
        assert_eq!(correct_uri, AnubisDB::default().build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_string());
        let _ = AnubisDB::default().run(host, tx).await.unwrap();
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    //TODO: should match VitaError not string message
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            AnubisDB::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
