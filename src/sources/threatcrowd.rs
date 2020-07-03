use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize)]
struct ThreatCrowdResult {
    subdomains: Option<Vec<String>>,
}

fn build_url(host: &str) -> String {
    format!(
        "https://www.threatcrowd.org/searchApi/v2/domain/report/?domain={}",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut results: HashSet<String> = HashSet::new();
    let uri = build_url(&host);
    let resp: ThreatCrowdResult = surf::get(uri).recv_json().await?;

    match resp.subdomains {
        Some(data) => data.into_iter().map(|s| results.insert(s)).for_each(drop),

        None => eprintln!("ThreatCrowd couldn't find results for:{}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 5);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
