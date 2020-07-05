use crate::error::{Error, Result};
use crate::IntoSubdomain;
use serde_json::value::Value;
use std::collections::HashSet;
use std::sync::Arc;

struct AnubisResult {
    results: Value,
}

impl AnubisResult {
    fn new(results: Value) -> Self {
        Self { results }
    }
}

impl IntoSubdomain for AnubisResult {
    fn subdomains(&self) -> HashSet<String> {
        self.results
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().into())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://jldc.me/anubis/subdomains/{}", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let resp: Option<Value> = surf::get(uri).recv_json().await?;

    match resp {
        Some(d) => {
            let subdomains = AnubisResult::new(d).subdomains();

            if !subdomains.is_empty() {
                Ok(subdomains)
            } else {
                Err(Error::source_error("AnubisDB", host))
            }
        }

        None => Err(Error::source_error("AnubisDB", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://jldc.me/anubis/subdomains/hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_string());
        let results = run(host).await.unwrap();
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "AnubisDB couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
