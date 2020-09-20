use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::header::AUTHORIZATION;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

struct Creds {
    key: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("CHAOS_KEY") {
            Ok(key) => Ok(Self { key }),
            Err(_) => Err(Error::key_error("Chaos", &["CHAOS_KEY"])),
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

fn build_url(host: &str) -> String {
    format!("https://dns.projectdiscovery.io/dns/{}/subdomains", host)
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from projectdiscovery choas for: {}", &host);
    let api_key = match Creds::read_creds() {
        Ok(creds) => creds.key,
        Err(e) => return Err(e),
    };

    //TODO: add info on if authenticaiton failed.
    let uri = build_url(&host);
    let resp = client
        .get(&uri)
        .header(AUTHORIZATION, api_key)
        .send()
        .await?;
    //debug!("projectdiscovery chaos response: {:#?}", &resp);

    if resp.status().is_client_error() {
        warn!("got status: {} from chaos", resp.status().as_str());
        Err(Error::auth_error("chaos"))
    } else {
        let resp: ChaosResult = resp.json().await?;
        let subdomains = resp.subdomains();
        if !subdomains.is_empty() {
            info!("Discovered {} results for: {}", &subdomains.len(), &host);
            let _ = sender.send(subdomains).await?;
            Ok(())
        } else {
            warn!("No results for: {}", &host);
            Err(Error::source_error("Chaos", host))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;
    use tokio::sync::mpsc::channel;

    // Ignore, passed locally.
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = run(client!(), host, tx).await;
        assert!(!rx.recv().await.unwrap().is_empty());
    }

    // Ignore, passed locally.
    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!(25, 25);
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Chaos couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
