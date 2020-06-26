use serde::Deserialize;
use std::collections::HashSet;
// this is replicated in manyt places
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Deserialize)]
struct ThreatminerResult {
    results: Vec<String>,
}

pub fn build_url(host: &str) -> String {
    format!(
        "https://api.threatminer.org/v2/domain.php?q={}&api=True&rt=5",
        host
    )
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let mut results = HashSet::new();
    let resp: Option<ThreatminerResult> = surf::get(uri).recv_json().await?;
    match resp {
        Some(d) => d
            .results
            .into_iter()
            .map(|s| results.insert(s))
            .for_each(drop),
        None => eprintln!("Threatminer couldn't find any results for: {}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.threatminer.org/v2/domain.php?q=hackerone.com&api=True&rt=5";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let results = run("hackerone.com".to_owned()).await.unwrap();
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "hdsad.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() == 0);
    }
}
