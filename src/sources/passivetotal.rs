use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::header::ACCEPT;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct Creds {
    key: String,
    secret: String,
}

impl Creds {
    fn from_env() -> Result<Self> {
        dotenv().ok();
        let key = env::var("PASSIVETOTAL_KEY");
        let secret = env::var("PASSIVETOTAL_SECRET");
        match (key, secret) {
            (Ok(k), Ok(s)) => Ok(Self { key: k, secret: s }),
            _ => Err(VitaError::UnsetKeys(vec![
                "PASSIVETOTAL_KEY".into(),
                "PASSIVETOTAL_SECRET".into(),
            ])),
        }
    }
}

#[derive(Serialize)]
struct Query {
    query: String,
}

impl Query {
    fn new(host: &str) -> Self {
        Self {
            query: host.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PassiveTotalResult {
    success: bool,
    #[serde(rename = "primaryDomain")]
    primary_domain: String,
    subdomains: Vec<String>,
}

impl IntoSubdomain for PassiveTotalResult {
    fn subdomains(&self) -> Vec<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.primary_domain))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct PassiveTotal {
    client: Client,
}

impl PassiveTotal {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self) -> String {
        "https://api.passivetotal.org/v2/enrichment/subdomains".to_string()
    }
}

#[async_trait]
impl DataSource for PassiveTotal {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from passivetotal for: {}", &host);
        let creds = match Creds::from_env() {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        let uri = self.build_url();
        let query = Query::new(&host);
        let resp = self
            .client
            .get(&uri)
            .basic_auth(&creds.key, Some(&creds.secret))
            .header(ACCEPT, "application/json")
            .json(&query)
            .send()
            .await?;

        if resp.status().is_client_error() {
            warn!("got status: {} from passivetotal", resp.status().as_str());
            return Err(VitaError::AuthError("Passivetotal".into()));
        } else {
            let resp: PassiveTotalResult = resp.json().await?;
            let subdomains = resp.subdomains();

            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("no results for {} from PassiveTotal", &host);
        Err(VitaError::SourceError("PassiveTotal".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = PassiveTotal::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            PassiveTotal::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
