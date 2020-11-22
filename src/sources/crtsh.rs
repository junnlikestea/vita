use crate::error::{Error, Result};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, trace, warn};

#[derive(Deserialize, Hash, PartialEq, Debug, Eq)]
struct CrtshResult {
    name_value: String,
}

impl IntoSubdomain for Vec<CrtshResult> {
    fn subdomains(&self) -> Vec<String> {
        self.iter().map(|s| s.name_value.to_owned()).collect()
    }
}

#[derive(Default)]
pub struct Crtsh {
    client: Client,
}

impl Crtsh {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str) -> String {
        format!("https://crt.sh/?q=%.{}&output=json", host)
    }
}

#[async_trait]
impl DataSource for Crtsh {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from crt.sh for: {}", &host);
        let uri = self.build_url(&host);
        let resp: Option<Vec<CrtshResult>> = self.client.get(&uri).send().await?.json().await?;
        debug!("crt.sh response: {:?}", &resp);

        match resp {
            Some(data) => {
                let subdomains = data.subdomains();
                info!("Discovered {} results for: {}", subdomains.len(), &host);
                let _ = tx.send(subdomains).await?;
                Ok(())
            }
            None => {
                warn!("No results for: {}", &host);
                Err(Error::source_error("Crt.sh", host))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://crt.sh/?q=%.hackerone.com&output=json";
        assert_eq!(correct_uri, Crtsh::default().build_url("hackerone.com"));
    }

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = Crtsh::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[ignore] // tests passing locally but failing on linux ci?
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = Crtsh::default().run(host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Crt.sh couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
