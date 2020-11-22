use crate::error::{Error, Result};
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
    key: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("C99_KEY") {
            Ok(key) => Ok(Self { key }),
            Err(_) => Err(Error::key_error("C99", &["C99_KEY"])),
        }
    }
}

#[derive(Debug, Deserialize)]
struct C99Result {
    subdomains: Option<Vec<C99Item>>,
}

#[derive(Debug, Deserialize)]
struct C99Item {
    subdomain: String,
}

impl IntoSubdomain for C99Result {
    fn subdomains(&self) -> Vec<String> {
        self.subdomains
            .iter()
            .flatten()
            .map(|s| s.subdomain.to_string())
            .collect()
    }
}

#[derive(Default)]
pub struct C99 {
    client: Client,
}

impl C99 {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str, api_key: &str) -> String {
        format!(
            "https://api.c99.nl/subdomainfinder?key={}&domain={}&json",
            api_key, host
        )
    }
}

#[async_trait]
impl DataSource for C99 {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from C99 for: {}", &host);
        let api_key = match Creds::read_creds() {
            Ok(creds) => creds.key,
            Err(e) => return Err(e),
        };

        let uri = self.build_url(&host, &api_key);
        let resp = self.client.get(&uri).send().await?;

        //TODO: not sure about this logic.
        if resp.status().is_client_error() {
            warn!(
                "got status: {} from c99, you may have hit rate limits",
                resp.status().as_str()
            );
            Err(Error::auth_error("c99"))
        } else {
            let resp: C99Result = resp.json().await?;
            let subdomains = resp.subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await?;
                Ok(())
            } else {
                warn!("No results for: {}", &host);
                Err(Error::source_error("C99", host))
            }
        }
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
        let _ = C99::default().run(host, tx).await;
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
        let res = C99::default().run(host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "C99 couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
