use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::header::AUTHORIZATION;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct Creds {
    key: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("CHAOS_KEY") {
            Ok(key) => Ok(Self { key }),
            Err(_) => Err(VitaError::UnsetKeys(vec!["CHAOS_KEY".into()])),
        }
    }
}

#[derive(Deserialize, Debug)]
struct ChaosResult {
    domain: String,
    subdomains: Vec<String>,
}

impl IntoSubdomain for ChaosResult {
    fn subdomains(&self) -> Vec<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.domain))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct Chaos {
    client: Client,
}

impl Chaos {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!("https://dns.projectdiscovery.io/dns/{}/subdomains", host)
    }
}

#[async_trait]
impl DataSource for Chaos {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from projectdiscovery choas for: {}", &host);
        let api_key = match Creds::read_creds() {
            Ok(creds) => creds.key,
            Err(e) => return Err(e),
        };
        let uri = self.build_url(&host);
        let resp = self
            .client
            .get(&uri)
            .header(AUTHORIZATION, api_key)
            .send()
            .await?;

        if resp.status().is_client_error() {
            warn!("got status: {} from chaos", resp.status().as_str());
            return Err(VitaError::AuthError("Chaos".into()));
        } else {
            let resp: ChaosResult = resp.json().await?;
            let subdomains = resp.subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("no results for {} from Chaos", &host);
        Err(VitaError::SourceError("Chaos".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    // Ignore, passed locally.
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = Chaos::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    // Ignore, passed locally.
    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            Chaos::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
