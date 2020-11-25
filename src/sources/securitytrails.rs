use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct Creds {
    api_key: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("SECURITY_TRAILS_KEY") {
            Ok(api_key) => Ok(Self { api_key }),
            Err(_) => Err(VitaError::UnsetKeys(vec!["SECURITY_TRAILS_KEY".into()])),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct SecTrailsResult {
    subdomains: Vec<String>,
    #[serde(skip)]
    host: Arc<String>,
}

impl IntoSubdomain for SecTrailsResult {
    fn subdomains(&self) -> Vec<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.host))
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct SecurityTrails {
    client: Client,
}

impl SecurityTrails {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!(
            "https://api.securitytrails.com/v1/domain/{}/subdomains",
            host
        )
    }
}

#[async_trait]
impl DataSource for SecurityTrails {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from securitytrails for: {}", &host);

        let api_key = match Creds::read_creds() {
            Ok(creds) => creds.api_key,
            Err(e) => return Err(e),
        };

        let uri = self.build_url(&host);
        let resp = self
            .client
            .get(&uri)
            .header("apikey", api_key)
            .send()
            .await?;

        if resp.status().is_client_error() {
            warn!(
                "got status: {} from security trails",
                resp.status().as_str()
            );
            return Err(VitaError::AuthError("SecurityTrails".into()));
        } else {
            let resp: Option<SecTrailsResult> = resp.json().await?;
            if let Some(data) = resp {
                let subdomains = data.subdomains();
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("no results for {} from SecurityTrails", &host);
        Err(VitaError::SourceError("SecurityTrails".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.securitytrails.com/v1/domain/hackerone.com/subdomains";
        assert_eq!(
            correct_uri,
            SecurityTrails::default().build_url("hackerone.com")
        );
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = SecurityTrails::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    // TODO: Test assumes credentials from env are valid.
    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            SecurityTrails::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
