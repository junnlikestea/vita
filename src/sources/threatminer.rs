use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize)]
struct ThreatminerResult {
    results: Vec<String>,
}

impl IntoSubdomain for ThreatminerResult {
    //todo: does it have to be HashSet<String> or can we change to HashSet<&str>
    fn subdomains(&self) -> HashSet<String> {
        self.results.iter().map(|s| s.into()).collect()
    }
}

pub fn build_url(host: &str) -> String {
    format!(
        "https://api.threatminer.org/v2/domain.php?q={}&api=True&rt=5",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from threatminer for: {}", &host);
    let uri = build_url(&host);
    let resp: Option<ThreatminerResult> = client.get(&uri).send().await?.json().await?;

    match resp {
        Some(d) => {
            let subdomains = d.subdomains();

            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                Ok(subdomains)
            } else {
                warn!("No results found for: {}", &host);
                Err(Error::source_error("ThreatMiner", host))
            }
        }

        None => Err(Error::source_error("ThreatMiner", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.threatminer.org/v2/domain.php?q=hackerone.com&api=True&rt=5";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    //Some("WaybackMachine couldn't find results for: anVubmxpa2VzdGVh.com")
    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "ThreatMiner couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
