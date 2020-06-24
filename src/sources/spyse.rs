use dotenv::dotenv;
use http_types::headers;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
// this is replicated in manyt places
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

fn build_url(host: &str) -> String {
    format!(
        "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain={}",
        host
    )
}

pub async fn run(host: &str) -> Result<HashSet<String>> {
    // should this process be done with lazy_static macro? otherwise we would be
    // creatng this for every call to run
    dotenv().ok();
    let api_token = env::var("SPYSE_TOKEN")
        .expect("SPYSE_TOKEN must be set in order to use Spyse as a data source");
    let uri = build_url(host);
    let mut subdomains = HashSet::new();
    let resp: Option<SpyseResult> = surf::get(uri)
        .set_header(headers::ACCEPT, "application/json")
        .set_header(headers::AUTHORIZATION, format!("Bearer {}", api_token))
        .recv_json()
        .await?;

    match resp {
        Some(d) => d
            .data
            .items
            .into_iter()
            .map(|i| subdomains.insert(i.name))
            .for_each(drop),
        None => eprintln!("Spyse coudln't find any results for: {}", &host),
    }

    Ok(subdomains)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let results = run("hackerone.com").await.unwrap();
        assert!(results.len() > 3);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "hdsad.com";
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
