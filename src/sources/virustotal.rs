use async_std::{prelude::*, task};
use serde::Deserialize;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Deserialize, Hash, PartialEq, Eq)]
struct Subdomain {
    id: String,
}

#[derive(Deserialize, Hash, PartialEq, Eq)]
struct VirustotalResult {
    data: Vec<Subdomain>,
}

fn build_url(host: &str) -> String {
    format!(
        "https://www.virustotal.com/ui/domains/{}/subdomains?limit=40",
        host
    )
}

pub async fn run(host: &str) -> Result<HashSet<String>> {
    let mut results: HashSet<String> = HashSet::new();
    let uri = build_url(host);
    //TODO: add error handling on response. We want to handle errors gracefully, not panic on
    // deserializing empty json body.
    let resp: VirustotalResult = surf::get(uri).recv_json().await?;
    resp.data
        .into_iter()
        .map(|s| results.insert(s.id))
        .for_each(drop);
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn vt_returns_results() {
        let host = "hackerone.com";
        let results = run(host).await.unwrap();
        assert!(results.len() > 5);
    }
}
