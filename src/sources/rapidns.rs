use crate::error::{Error, Result};
use crate::IntoSubdomain;
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug)]
struct RapidnsResponse {
    body: String,
}

impl RapidnsResponse {
    pub fn new(body: String) -> Self {
        Self { body }
    }
}

impl IntoSubdomain for RapidnsResponse {
    fn subdomains(&self) -> HashSet<String> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r#"target="_blank">(.*)</a>"#).unwrap();
        }

        RE.captures_iter(&self.body)
            .map(|s| s[1].to_string())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://rapiddns.io/subdomain/{}?full=1", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let body = surf::get(uri).recv_string().await?;
    let subdomains = RapidnsResponse::new(body).subdomains();

    if !subdomains.is_empty() {
        Ok(subdomains)
    } else {
        Err(Error::source_error("RapidDNS", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_string());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }
}
