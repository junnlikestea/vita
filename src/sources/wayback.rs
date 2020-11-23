use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::value::Value;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, trace, warn};
use url::Url;

struct WaybackResult {
    data: Value,
}

impl WaybackResult {
    fn new(data: Value) -> Self {
        Self { data }
    }
}

//TODO: this could be cleaned up, to avoid creating the extra vec `vecs`
impl IntoSubdomain for WaybackResult {
    fn subdomains(&self) -> Vec<String> {
        let arr = self.data.as_array().unwrap();
        let vecs: Vec<&str> = arr.iter().map(|s| s[0].as_str().unwrap()).collect();
        vecs.into_iter()
            .filter_map(|a| Url::parse(a).ok())
            .map(|u| u.host_str().unwrap().into())
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct Wayback {
    client: Client,
}

impl Wayback {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!(
            "https://web.archive.org/cdx/search/cdx?url=*.{}/*&output=json\
    &fl=original&collapse=urlkey&limit=100000",
            host
        )
    }
}

#[async_trait]
impl DataSource for Wayback {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from wayback for: {}", &host);
        let uri = self.build_url(&host);
        let resp: Option<Value> = self.client.get(&uri).send().await?.json().await?;

        if let Some(data) = resp {
            let subdomains = WaybackResult::new(data).subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("no results found for {} from Wayback Machine", &host);
        Err(VitaError::SourceError("Wayback Machine".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://web.archive.org/cdx/search/cdx?url=*.hackerone.com/*&output=json\
    &fl=original&collapse=urlkey&limit=100000";
        assert_eq!(correct_uri, Wayback::default().build_url("hackerone.com"));
    }

    #[ignore] // hangs forever on windows for some reasons?
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(20);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = Wayback::default().run(host, tx).await;
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
            Wayback::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
