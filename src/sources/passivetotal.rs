use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::header::ACCEPT;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

struct Creds {
    key: String,
    secret: String,
}

impl Creds {
    fn from_env() -> Result<Self> {
        dotenv().ok();
        let key = env::var("PASSIVETOTAL_KEY");
        let secret = env::var("PASSIVETOTAL_SECRET");
        if key.is_ok() && secret.is_ok() {
            Ok(Self {
                key: key?,
                secret: secret?,
            })
        } else {
            Err(Error::key_error(
                "PassiveTotal",
                &["PASSIVETOTAL_KEY", "PASSIVETOTAL_SECRET"],
            ))
        }
    }
}

#[derive(Serialize)]
struct Query {
    query: String,
}

impl Query {
    fn new(host: &str) -> Self {
        Self {
            query: host.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PassiveTotalResult {
    success: bool,
    #[serde(rename = "primaryDomain")]
    primary_domain: String,
    subdomains: Vec<String>,
}

impl IntoSubdomain for PassiveTotalResult {
    fn subdomains(&self) -> HashSet<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.primary_domain))
            .collect()
    }
}

fn build_url() -> String {
    "https://api.passivetotal.org/v2/enrichment/subdomains".to_string()
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from passivetotal for: {}", &host);
    let creds = match Creds::from_env() {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    let uri = build_url();
    let query = Query::new(&host);
    let resp: PassiveTotalResult = client
        .get(&uri)
        .basic_auth(&creds.key, Some(&creds.secret))
        .header(ACCEPT, "application/json")
        .json(&query)
        .send()
        .await?
        .json()
        .await?;

    debug!("passivetotal response: {:?}", &resp);
    if resp.success {
        let subdomains = resp.subdomains();

        if !subdomains.is_empty() {
            info!("Discovered {} results for: {}", &subdomains.len(), &host);
            Ok(subdomains)
        } else {
            warn!("No results for: {}", &host);
            Err(Error::source_error("PassiveTotal", host))
        }
    } else {
        Err(Error::source_error("PassiveTotal", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "PassiveTotal couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
