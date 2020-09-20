use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::header::ACCEPT;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

struct Creds {
    token: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("SPYSE_TOKEN") {
            Ok(token) => Ok(Self { token }),
            Err(_) => Err(Error::key_error("Spyse", &["SPYSE_TOKEN"])),
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

fn build_url(host: &str) -> String {
    format!(
        "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain={}",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from spyse for: {}", &host);
    let token = match Creds::read_creds() {
        Ok(creds) => creds.token,
        Err(e) => return Err(e),
    };

    let uri = build_url(&host);
    let resp = client
        .get(&uri)
        .header(ACCEPT, "application/json")
        .bearer_auth(token)
        .send()
        .await?;

    if resp.status().is_client_error() {
        warn!("got status: {} from spyse", resp.status().as_str());
        Err(Error::auth_error("Spyse"))
    } else {
        let resp: Option<SpyseResult> = resp.json().await?;

        if resp.is_some() {
            let subdomains = resp.unwrap().subdomains();
            info!("Discovered {} results for {}", &subdomains.len(), &host);
            sender.send(subdomains).await?;
            Ok(())
        } else {
            warn!("No results for: {}", &host);
            Err(Error::source_error("Spyse", host))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!(25, 25);
        let _ = run(client, host, tx).await;
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
        let client = client!(25, 25);
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Spyse couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
