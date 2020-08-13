use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
struct RecondevResult {
    #[serde(rename = "rawDomains")]
    raw_domains: Vec<String>,
}

impl IntoSubdomain for Vec<RecondevResult> {
    fn subdomains(&self) -> HashSet<String> {
        self.iter()
            .flat_map(|s| s.raw_domains.iter())
            .map(|d| d.to_owned())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://api.recon.dev/search?domain={}", host)
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from api.recon.dev for: {}", &host);
    let uri = build_url(&host);
    let resp: Option<Vec<RecondevResult>> = client.get(&uri).send().await?.json().await?;
    debug!("api.recon.dev response:{:?}", &resp);

    if let Some(r) = resp {
        Ok(r.subdomains())
    } else {
        Err(Error::source_error("ReconDev", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.recon.dev/search?domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("uber.com".to_string());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn handles_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let results = run(client, host).await;
        let e = results.unwrap_err();
        assert_eq!(
            e.to_string(),
            "ReconDev couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
