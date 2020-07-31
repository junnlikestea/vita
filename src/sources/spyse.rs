use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use http_types::headers;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

#[derive(Deserialize)]
struct SpyseResult {
    data: SpyseItem,
}

#[derive(Deserialize)]
struct SpyseItem {
    items: Vec<Subdomain>,
}

#[derive(Deserialize)]
struct Subdomain {
    name: String,
}

impl IntoSubdomain for SpyseResult {
    fn subdomains(&self) -> HashSet<String> {
        self.data.items.iter().map(|i| i.name.to_owned()).collect()
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain={}",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    // TODO:// handle pagnation?
    dotenv().ok();
    let api_token = match env::var("SPYSE_TOKEN") {
        Ok(key) => key,
        Err(_) => return Err(Error::key_error("Spyse")),
    };

    let uri = build_url(&host);
    let resp: Option<SpyseResult> = surf::get(uri)
        .set_header(headers::ACCEPT, "application/json")
        .set_header(headers::AUTHORIZATION, format!("Bearer {}", api_token))
        .recv_json()
        .await?;

    match resp {
        Some(d) => {
            let subdomains = d.subdomains();
            if !subdomains.is_empty() {
                Ok(subdomains)
            } else {
                Err(Error::source_error("Spyse", host))
            }
        }

        None => Err(Error::source_error("Spyse", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

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
            "Spyse couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
