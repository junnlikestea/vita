use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use http_types::headers::AUTHORIZATION;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

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

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    dotenv().ok();
    let api_key = match env::var("CHAOS_KEY") {
        Ok(key) => key,
        Err(_) => return Err(Error::key_error("Chaos")),
    };

    let uri = build_url(&host);
    let resp: ChaosResult = surf::get(uri)
        .set_header(AUTHORIZATION, api_key)
        .recv_json()
        .await?;
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

    // Ignore, passed locally.
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    // Ignore, passed locally.
    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Chaos couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
