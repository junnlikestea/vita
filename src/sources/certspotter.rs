use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct CertSpotterResult {
    dns_names: Vec<String>,
}

impl IntoSubdomain for Vec<CertSpotterResult> {
    fn subdomains(&self) -> HashSet<String> {
        self.iter()
            .flat_map(|d| d.dns_names.iter())
            .map(|s| s.into())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://api.certspotter.com/v1/issuances?domain={}\
        &include_subdomains=true&expand=dns_names",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from certspotter for: {}", &host);
    let uri = build_url(&host);
    let resp: Option<Vec<CertSpotterResult>> = client.get(&uri).send().await?.json().await?;
    debug!("certspotter response: {:?}", &resp);

    match resp {
        Some(data) => {
            let subdomains = data.subdomains();

            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                Ok(subdomains)
            } else {
                warn!("No results for: {}", &host);
                Err(Error::source_error("CertSpotter", host))
            }
        }
        _ => Err(Error::source_error("CertSpotter", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.certspotter.com/v1/issuances?domain=hackerone.com\
        &include_subdomains=true&expand=dns_names";
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

    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "CertSpotter couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
