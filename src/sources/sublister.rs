use crate::IntoSubdomain;
use crate::Result;
use serde_json::value::Value;
use std::collections::HashSet;
use std::sync::Arc;
use std::{error::Error, fmt};

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

#[derive(Debug)]
struct SublisterError {
    host: Arc<String>,
}

impl SublisterError {
    fn new(host: Arc<String>) -> Self {
        Self { host }
    }
}

impl Error for SublisterError {}

impl fmt::Display for SublisterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Sublister couldn't find any results for: {}", self.host)
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
            if subdomains.len() != 0 {
                Ok(subdomains)
            } else {
                Err(Box::new(SublisterError::new(host)))
            }
        }

        None => Err(Box::new(SublisterError::new(host))),
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
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Sublister couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
