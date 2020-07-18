use crate::error::Error;
use crate::error::Result;
use crate::IntoSubdomain;
use async_std::task;
use dotenv::dotenv;
use http_types::StatusCode;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

#[derive(Deserialize)]
struct BinaryEdgeResponse {
    page: i32,
    pagesize: i32,
    total: i32,
    events: Vec<String>,
}

impl IntoSubdomain for BinaryEdgeResponse {
    fn subdomains(&self) -> HashSet<String> {
        self.events.iter().map(|s| s.into()).collect()
    }
}

fn build_url(host: &str, page: Option<i32>) -> String {
    match page {
        Some(p) => format!(
            "https://api.binaryedge.io/v2/query/domains/subdomain/{}?page={}",
            host, p
        ),
        None => format!(
            "https://api.binaryedge.io/v2/query/domains/subdomain/{}",
            host
        ),
    }
}

// fetches the page in sequential order, it would be better to fetch them concurrently,
// but for the small amount of pages it probably doesn't matter
pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut tasks = Vec::new();
    let mut results: HashSet<String> = HashSet::new();
    let resp = next_page(host.clone(), None).await?;
    // insert subdomains from first page.
    resp.subdomains()
        .into_iter()
        .map(|s| results.insert(s))
        .for_each(drop);
    let mut page = resp.page;

    loop {
        let host = host.clone();

        if page > 0 && page * resp.pagesize >= resp.total {
            break;
        }

        page += 1;
        tasks.push(task::spawn(
            async move { next_page(host, Some(page)).await },
        ));
    }

    for t in tasks {
        t.await?
            .subdomains()
            .into_iter()
            .map(|s| results.insert(s))
            .for_each(drop);
    }

    Ok(results)
}

async fn next_page(host: Arc<String>, page: Option<i32>) -> Result<BinaryEdgeResponse> {
    dotenv().ok();
    let uri = build_url(&host, page);
    let api_key = env::var("BINARYEDGE_TOKEN")
        .expect("BINARYEDGE_TOKEN must be set in order to use Binaryedge as a data source");
    let mut resp = surf::get(uri).set_header("X-Key", api_key).await?;

    // Should probably add cleaner match arms, but this will do for now.
    match resp.status() {
        StatusCode::Unauthorized | StatusCode::Forbidden => Err(Error::auth_error("BinaryEdge")),
        _ => {
            let be: BinaryEdgeResponse = resp.body_json().await?;
            Ok(be)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Tests passed locally, ignoring for now.
    // TODO: Add github secret to use ignored tests
    // Checks to see if the run function returns subdomains
    #[async_test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_string());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[async_test]
    #[ignore]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "BinaryEdge couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }

    #[async_test]
    #[ignore]
    async fn handle_auth_error() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Couldn't authenticate or have hit rate-limits to the BinaryEdge API"
        );
    }
}
