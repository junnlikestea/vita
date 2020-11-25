use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::value::Value;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct SublisterResult {
    items: Vec<Value>,
}

impl SublisterResult {
    fn new(items: Vec<Value>) -> Self {
        SublisterResult { items }
    }
}

//TODO: can this just be collected without the map?
impl IntoSubdomain for SublisterResult {
    fn subdomains(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct Sublister {
    client: Client,
}

impl Sublister {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!("https://api.sublist3r.com/search.php?domain={}", host)
    }
}

#[async_trait]
impl DataSource for Sublister {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from sublister for: {}", &host);
        let uri = self.build_url(&host);
        let resp: Option<Value> = self.client.get(&uri).send().await?.json().await?;

        if let Some(data) = resp {
            let subdomains = SublisterResult::new(data.as_array().unwrap().to_owned()).subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("no results for {} from Sublist3r", &host);
        Err(VitaError::SourceError("Sublist3r".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.sublist3r.com/search.php?domain=hackerone.com";
        assert_eq!(correct_uri, Sublister::default().build_url("hackerone.com"));
    }

    //TODO: tweak test for GithubActions (passed locally)
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = Sublister::default().run(host, tx).await;
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
            Sublister::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
