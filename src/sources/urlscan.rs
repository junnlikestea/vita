use serde::Deserialize;
use std::collections::HashSet;

// this is replicated in manyt places
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Deserialize)]
struct UrlScanResult {
    results: HashSet<UrlScanPage>,
}

#[derive(Deserialize, Hash, Eq, PartialEq)]
struct UrlScanPage {
    page: UrlScanDomain,
}

#[derive(Deserialize, Eq, Hash, PartialEq)]
struct UrlScanDomain {
    domain: String,
}

fn build_url(host: &str) -> String {
    format!("https://urlscan.io/api/v1/search/?q=domain:{}", host)
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let mut subdomains = HashSet::new();
    let resp: Option<UrlScanResult> = surf::get(uri).recv_json().await?;
    // why loop through twice? and create two maps, we could just use collect on a successful
    // result and return?
    match resp {
        Some(data) => data
            .results
            .into_iter()
            .map(|s| subdomains.insert(s.page.domain))
            .for_each(drop),
        None => eprintln!("no results"),
    }

    Ok(subdomains)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://urlscan.io/api/v1/search/?q=domain:hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "anVubmxpa2VzdGVh.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }

    #[async_test]
    async fn returns_results() {
        let results = run("hackerone.com".to_owned()).await.unwrap();
        assert!(results.len() > 3);
    }
}
