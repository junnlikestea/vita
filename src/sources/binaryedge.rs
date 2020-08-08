use crate::error::Error;
use crate::error::Result;
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

struct Creds {
    token: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("BINARYEDGE_TOKEN") {
            Ok(token) => Ok(Self { token }),
            Err(_) => Err(Error::key_error("BinaryEdge", &["BINARYEDGE_TOKEN"])),
        }
    }
}

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
// but for the small amount of pages it probably doesn't matte
pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    let mut tasks = Vec::new();
    let mut results: HashSet<String> = HashSet::new();
    let resp = next_page(client.clone(), host.clone(), None).await?;

    // insert subdomains from first page.
    resp.subdomains()
        .into_iter()
        .map(|s| results.insert(s))
        .for_each(drop);
    let mut page = resp.page;

    loop {
        let host = host.clone();
        let client = client.clone();

        if page > 0 && page * resp.pagesize >= resp.total {
            break;
        }

        page += 1;
        tasks.push(tokio::task::spawn(async move {
            next_page(client, host, Some(page)).await
        }));
    }

    for t in tasks {
        t.await??
            .subdomains()
            .into_iter()
            .map(|s| results.insert(s))
            .for_each(drop);
    }

    Ok(results)
}

async fn next_page(
    client: Client,
    host: Arc<String>,
    page: Option<i32>,
) -> Result<BinaryEdgeResponse> {
    trace!("fetching a page from binaryedge for: {}", &host);
    let uri = build_url(&host, page);

    let token = match Creds::read_creds() {
        Ok(creds) => creds.token,
        Err(e) => return Err(e),
    };

    let resp = client.get(&uri).header("X-Key", token).send().await?;

    // Should probably add cleaner match arms, but this will do for now.
    if resp.status().is_success() {
        debug!("binaryedge response: {:?}", &resp);
        let be: BinaryEdgeResponse = resp.json().await?;
        Ok(be)
    } else {
        info!("binaryedge returned authentication error");
        Err(Error::auth_error("BinaryEdge"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    // Tests passed locally, ignoring for now.
    // TODO: Add github secret to use ignored tests
    // Checks to see if the run function returns subdomains
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_string());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "BinaryEdge couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn handle_auth_error() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Couldn't authenticate or have hit rate-limits to the BinaryEdge API"
        );
    }
}
