use crate::error::{Result, VitaError};
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, trace, warn};

struct Creds {
    url: String,
    api_key: String,
}

impl Creds {
    fn read_creds() -> Result<Self> {
        dotenv().ok();
        let api_key = env::var("INTELX_KEY");
        let url = env::var("INTELX_URL");

        match (api_key, url) {
            (Ok(k), Ok(u)) => Ok(Self { url: u, api_key: k }),
            _ => Err(VitaError::UnsetKeys(vec![
                "INTELX_URL".into(),
                "INTELX_KEY".into(),
            ])),
        }
    }
}

#[derive(Deserialize, Debug)]
struct SearchId {
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Query {
    term: String,
    maxresults: i32,
    media: usize,
    target: usize,
    timeout: i32,
}

impl Query {
    fn new(term: String) -> Self {
        Self {
            term,
            maxresults: 100000,
            media: 0,
            target: 1,
            timeout: 20,
        }
    }
}

#[derive(Deserialize, Debug)]
struct IntelxItem {
    selectorvalue: String,
}

#[derive(Deserialize, Debug)]
struct IntelxResults {
    selectors: Vec<IntelxItem>,
    status: usize,
}

impl IntoSubdomain for IntelxResults {
    fn subdomains(&self) -> Vec<String> {
        self.selectors
            .iter()
            .map(|s| s.selectorvalue.to_owned())
            .collect()
    }
}

#[derive(Default, Clone)]
pub struct Intelx {
    client: Client,
}

impl Intelx {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(
        &self,
        intelx_url: &str,
        api_key: &str,
        querying: bool,
        search_id: Option<&str>,
    ) -> String {
        if querying {
            format!("https://{}/phonebook/search?k={}", intelx_url, api_key)
        } else {
            format!(
                "https://{}/phonebook/search/result?k={}&id={}&limit=100000",
                intelx_url,
                api_key,
                search_id.unwrap()
            )
        }
    }

    async fn get_searchid(&self, host: Arc<String>) -> Result<String> {
        trace!("getting intelx searchid");
        let creds = match Creds::read_creds() {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        let query_uri = self.build_url(&creds.url, &creds.api_key, true, None);
        let body = Query::new(host.to_string());
        let search_id: SearchId = self
            .client
            .post(&query_uri)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        debug!("searchid: {:?}", &search_id);
        Ok(search_id.id)
    }
}

#[async_trait]
impl DataSource for Intelx {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from intelx for: {}", &host);
        let creds = match Creds::read_creds() {
            Ok(creds) => creds,
            Err(e) => return Err(e),
        };

        let search_id = self.get_searchid(host.clone()).await?;
        let uri = self.build_url(&creds.url, &creds.api_key, false, Some(&search_id));
        let resp = self.client.get(&uri).send().await?;

        if resp.status().is_client_error() {
            warn!("got status: {} for intelx", resp.status().as_str());
            return Err(VitaError::AuthError("Intelx".into()));
        } else {
            let resp: IntelxResults = resp.json().await?;
            let subdomains = resp.subdomains();
            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                let _ = tx.send(subdomains).await;
                return Ok(());
            }
        }

        warn!("no results for {} from Intelx", &host);
        Err(VitaError::SourceError("Intelx".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    #[ignore]
    async fn search_id() {
        let host = Arc::new("hackerone.com".to_owned());
        let id = Intelx::default().get_searchid(host).await.unwrap();
        assert!(!id.is_empty())
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = Intelx::default().run(host, tx).await;
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
            Intelx::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
