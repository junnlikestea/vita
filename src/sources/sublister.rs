use crate::error::{Error, Result};
use crate::IntoSubdomain;
use serde_json::value::Value;
use std::collections::HashSet;
use std::sync::Arc;

struct SublisterResult {
    items: Vec<Value>,
}

impl SublisterResult {
    fn new(items: Vec<Value>) -> Self {
        SublisterResult { items }
    }
}

impl IntoSubdomain for SublisterResult {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://api.sublist3r.com/search.php?domain={}", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let resp: Option<Value> = surf::get(uri).recv_json().await?;

    match resp {
        Some(d) => {
            let subdomains = SublisterResult::new(d.as_array().unwrap().to_owned()).subdomains();
            if !subdomains.is_empty() {
                Ok(subdomains)
            } else {
                Err(Error::source_error("Sublist3r", host))
            }
        }

        None => Err(Error::source_error("Sublist3r", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.sublist3r.com/search.php?domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Sublist3r couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
