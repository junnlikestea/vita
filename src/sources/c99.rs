use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
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

fn build_url(host: &str, api_key: &str) -> String {
    format!(
        "https://api.c99.nl/subdomainfinder?key={}&domain={}&json",
        api_key, host
    )
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from C99 for: {}", &host);
    let api_key = match Creds::read_creds() {
        Ok(creds) => creds.key,
        Err(e) => return Err(e),
    };

    let uri = build_url(&host, &api_key);
    let resp = client.get(&uri).send().await?;

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
            let _ = sender.send(subdomains).await?;
            Ok(())
        } else {
            warn!("No results for: {}", &host);
            Err(Error::source_error("C99", host))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;
    use tokio::sync::mpsc::channel;

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let mut results = Vec::new();
        run(client, host, tx).await.unwrap();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "C99 couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
