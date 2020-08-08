use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::header::AUTHORIZATION;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

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
    fn subdomains(&self) -> HashSet<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.domain))
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://dns.projectdiscovery.io/dns/{}/subdomains", host)
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from projectdiscovery choas for: {}", &host);
    let api_key = match Creds::read_creds() {
        Ok(creds) => creds.key,
        Err(e) => return Err(e),
    };

    let uri = build_url(&host);
    let resp: ChaosResult = client
        .get(&uri)
        .header(AUTHORIZATION, api_key)
        .send()
        .await?
        .json()
        .await?;
    debug!("projectdiscovery chaos response: {:#?}", &resp);

    let subdomains = resp.subdomains();
    if !subdomains.is_empty() {
        Ok(subdomains)
    } else {
        Err(Error::source_error("Chaos", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    // Ignore, passed locally.
    // #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("yahoo.com".to_owned());
        let results = run(client!(), host).await.unwrap();
        for r in results.iter() {
            println!("{}", r);
        }
        println!("# of results:{}", results.len());
        assert!(!results.is_empty());
    }

    // Ignore, passed locally.
    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Chaos couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
