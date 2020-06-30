use crate::ResponseData;
use crate::Result;
use async_std::task;
use dotenv::dotenv;
use futures::future::join_all;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;

#[derive(Deserialize)]
struct BinaryEdgeResponse {
    page: i32,
    pagesize: i32,
    total: i32,
    events: Vec<String>,
}

impl ResponseData for BinaryEdgeResponse {
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.events
            .iter()
            .map(|s| map.insert(s.into()))
            .for_each(drop);
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
pub async fn run(host: String) -> Result<HashSet<String>> {
    let mut tasks = Vec::new();
    let mut results: HashSet<String> = HashSet::new();
    let resp = next_page(&host, None).await;
    // insert subdomains from first page.
    resp.subdomains(&mut results);
    let mut page = resp.page;

    loop {
        let host = host.clone();

        if page > 0 && page * resp.pagesize >= resp.total {
            break;
        }

        page += 1;
        tasks.push(task::spawn(
            async move { next_page(&host, Some(page)).await },
        ));
    }

    join_all(tasks)
        .await
        .iter()
        .map(|s| s.subdomains(&mut results))
        .for_each(drop);

    Ok(results)
}

async fn next_page(host: &str, page: Option<i32>) -> BinaryEdgeResponse {
    dotenv().ok();
    let uri = build_url(host, page);
    let api_key = env::var("BINARYEDGE_TOKEN")
        .expect("BINARYEDGE_TOKEN must be set in order to use Binaryedge as a data source");

    surf::get(uri)
        .set_header("X-Key", api_key)
        .recv_json()
        .await
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    #[async_test]
    #[ignore]
    async fn returns_results() {
        let results = run("example.com".to_owned()).await.unwrap();
        assert!(results.len() > 3);
    }
}
