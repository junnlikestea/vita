use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

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

#[derive(Debug, Deserialize)]
struct SecTrailsResult {
    subdomains: Vec<String>,
    #[serde(skip)]
    host: Arc<String>,
}

impl IntoSubdomain for SecTrailsResult {
    fn subdomains(&self) -> HashSet<String> {
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

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from securitytrails for: {}", &host);

    let api_key = match Creds::read_creds() {
        Ok(creds) => creds.api_key,
        Err(e) => return Err(e),
    };

    let uri = build_url(&host);
    let resp: Option<SecTrailsResult> = client
        .get(&uri)
        .header("apikey", api_key)
        .send()
        .await?
        .json()
        .await?;
    debug!("securitytrails response: {:?}", &resp);

    match resp {
        Some(d) => {
            let subdomains = d.subdomains();
            if !subdomains.is_empty() {
                Ok(subdomains)
            } else {
                Err(Error::source_error("SecurityTrails", host))
            }
        }

        None => Err(Error::source_error("SecurityTrails", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.securitytrails.com/v1/domain/hackerone.com/subdomains";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    // TODO: Test assumes credentials from env are valid.
    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "SecurityTrails couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
