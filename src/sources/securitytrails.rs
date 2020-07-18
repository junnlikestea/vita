use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

#[derive(Deserialize)]
struct SecTrailsResult {
    subdomains: Vec<String>,
    #[serde(skip)]
    host: Arc<String>,
}

impl IntoSubdomain for SecTrailsResult {
    fn subdomains(&self) -> HashSet<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.host))
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://api.securitytrails.com/v1/domain/{}/subdomains",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    dotenv().ok();
    let api_key = env::var("SECURITY_TRAILS_KEY")
        .expect("SECURITY_TRAILS_KEY must be set to use SecurityTrails API");
    let uri = build_url(&host);
    let resp: Option<SecTrailsResult> = surf::get(uri)
        .set_header("apikey", api_key)
        .recv_json()
        .await?;

    match resp {
        Some(d) => {
            let subdomains = d.subdomains();
            if !subdomains.is_empty() {
                Ok(subdomains)
            } else {
                Err(Error::source_error("SecurityTrails", host))
            }
        }

        None => Err(Error::source_error("SecurityTrails", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.securitytrails.com/v1/domain/hackerone.com/subdomains";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[ignore]
    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "SecurityTrails couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
