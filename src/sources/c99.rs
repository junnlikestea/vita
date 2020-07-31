use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

#[derive(Deserialize)]
struct C99Result {
    subdomains: Option<Vec<C99Item>>,
}

#[derive(Deserialize)]
struct C99Item {
    subdomain: String,
}

impl IntoSubdomain for C99Result {
    fn subdomains(&self) -> HashSet<String> {
        self.subdomains
            .iter()
            .flatten()
            .map(|s| s.subdomain.to_string())
            .collect()
    }
}

fn build_url(host: &str, api_key: &str) -> String {
    format!(
        "https://api.c99.nl/subdomainfinder?key={}&domain={}&json",
        api_key, host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    dotenv().ok();
    let api_key = env::var("C99_KEY").expect("C99_KEY must be set to use C99 as a data source");
    let uri = build_url(&host, &api_key);
    let resp: C99Result = surf::get(uri).recv_json().await?;
    let subdomains = resp.subdomains();

    if !subdomains.is_empty() {
        Ok(subdomains)
    } else {
        Err(Error::source_error("C99", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Checks to see if the run function returns subdomains

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "C99 couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
