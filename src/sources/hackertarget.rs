use crate::IntoSubdomain;
use crate::Result;
use std::collections::HashSet;
use std::sync::Arc;
use std::{error::Error, fmt};

const API_ERROR: &str = "error check your search parameter";

struct HackerTarget {
    items: String,
}

impl HackerTarget {
    fn new(items: String) -> Self {
        HackerTarget { items }
    }
}

impl IntoSubdomain for HackerTarget {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .lines()
            .map(|s| s.split(',').collect::<Vec<&str>>()[0].to_owned())
            .collect()
    }
}

#[derive(Debug)]
struct HackerTargetError {
    host: Arc<String>,
}

impl HackerTargetError {
    fn new(host: Arc<String>) -> Self {
        Self { host }
    }
}

impl Error for HackerTargetError {}

impl fmt::Display for HackerTargetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "HackerTarget couldn't find any results for: {}",
            self.host
        )
    }
}

fn build_url(host: &str) -> String {
    format!("https://api.hackertarget.com/hostsearch/?q={}", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let resp: String = surf::get(uri).recv_string().await?;

    if resp != API_ERROR {
        Ok(HackerTarget::new(resp).subdomains())
    } else {
        Err(Box::new(HackerTargetError::new(host)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 3);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "HackerTarget couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
