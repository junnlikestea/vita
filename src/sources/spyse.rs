use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::header::ACCEPT;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct Creds {
    token: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("SPYSE_TOKEN") {
            Ok(token) => Ok(Self { token }),
            Err(_) => Err(VitaError::UnsetKeys(vec!["SPYSE_TOKEN".into()])),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SpyseResult {
    data: SpyseItem,
}

#[derive(Debug, Deserialize)]
struct SpyseItem {
    items: Vec<Subdomain>,
}

#[derive(Debug, Deserialize)]
struct Subdomain {
    name: String,
}

impl IntoSubdomain for SpyseResult {
    fn subdomains(&self) -> Vec<String> {
        self.data.items.iter().map(|i| i.name.to_owned()).collect()
    }
}

#[derive(Default, Clone)]
pub struct Spyse {
    client: Client,
}

impl Spyse {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!(
            "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain={}",
            host
        )
    }
}

#[async_trait]
impl DataSource for Spyse {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from spyse for: {}", &host);
        let token = match Creds::read_creds() {
            Ok(creds) => creds.token,
            Err(e) => return Err(e),
        };

        let uri = self.build_url(&host);
        let resp = self
            .client
            .get(&uri)
            .header(ACCEPT, "application/json")
            .bearer_auth(token)
            .send()
            .await?;

        if resp.status().is_client_error() {
            warn!("got status: {} from spyse", resp.status().as_str());
            return Err(VitaError::AuthError("Spyse".into()));
        } else {
            let resp: Option<SpyseResult> = resp.json().await?;
            if let Some(data) = resp {
                let subdomains = data.subdomains();
                info!("Discovered {} results for {}", &subdomains.len(), &host);
                tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("No results for: {} from Spyse", &host);
        Err(VitaError::SourceError("Spyse".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain=hackerone.com";
        assert_eq!(correct_uri, Spyse::default().build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = Spyse::default().run(host, tx).await;
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
        let res = Spyse::default().run(host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Spyse couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
