use crate::error::{Error, Result};
use crate::IntoSubdomain;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize)]
struct Subdomain {
    id: String,
}

#[derive(Deserialize)]
struct VirustotalResult {
    data: Option<Vec<Subdomain>>,
}

impl IntoSubdomain for VirustotalResult {
    fn subdomains(&self) -> HashSet<String> {
        self.data
            .iter()
            .flatten()
            .map(|s| s.id.to_string())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    // TODO: can we gather the subdomains using:
    // Handle pagenation
    // https://www.virustotal.com/vtapi/v2/domain/report
    format!(
        "https://www.virustotal.com/ui/domains/{}/subdomains?limit=40",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let resp: VirustotalResult = surf::get(uri).recv_json().await?;
    let subdomains = resp.subdomains();

    if !subdomains.is_empty() {
        Ok(subdomains)
    } else {
        Err(Error::source_error("VirusTotal", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    // IGNORE by default since we have limited api calls.
    #[async_test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[ignore]
    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "VirusTotal couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
