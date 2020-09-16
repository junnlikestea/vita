use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

struct Creds {
    api_key: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("SECURITY_TRAILS_KEY") {
            Ok(api_key) => Ok(Self { api_key }),
            Err(_) => Err(Error::key_error("SecurityTrails", &["SECURITY_TRAILS_KEY"])),
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

fn build_url(host: &str) -> String {
    format!(
        "https://api.securitytrails.com/v1/domain/{}/subdomains",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from securitytrails for: {}", &host);

    let api_key = match Creds::read_creds() {
        Ok(creds) => creds.api_key,
        Err(e) => return Err(e),
    };

    let uri = build_url(&host);
    let resp = client.get(&uri).header("apikey", api_key).send().await?;
    if resp.status().is_client_error() {
        warn!(
            "got status: {} from security trails",
            resp.status().as_str()
        );
        Err(Error::auth_error("securitytrails"))
    } else {
        let resp: Option<SecTrailsResult> = resp.json().await?;

        if resp.is_some() {
            let subdomains = resp.unwrap().subdomains();
            info!("Discovered {} results for: {}", &subdomains.len(), &host);
            let _ = sender.send(subdomains).await?;
            Ok(())
        } else {
            warn!("No results for: {}", &host);
            Err(Error::source_error("SecurityTrails", host))
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
        let correct_uri = "https://api.securitytrails.com/v1/domain/hackerone.com/subdomains";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let mut results = Vec::new();
        run(client, host, tx).await;
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    // TODO: Test assumes credentials from env are valid.
    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "SecurityTrails couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
