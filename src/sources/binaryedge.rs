use crate::error::Result;
use crate::error::VitaError;
use crate::{DataSource, IntoSubdomain};
use async_trait::async_trait;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{info, trace, warn};

struct Creds {
    token: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        match env::var("BINARYEDGE_TOKEN") {
            Ok(token) => Ok(Self { token }),
            Err(_) => Err(VitaError::UnsetKeys(vec!["BINARYEDGE_TOKEN".into()])),
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
    fn subdomains(&self) -> Vec<String> {
        self.events.iter().map(|s| s.to_owned()).collect()
    }
}

#[derive(Default, Clone)]
pub struct BinaryEdge {
    client: Client,
}

impl BinaryEdge {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn build_url(&self, host: &str, page: Option<i32>) -> String {
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
}

//TODO: Clean this up, make pages fetch async
// fetches the page in sequential order, it would be better to fetch them concurrently,
// but for the small amount of pages it probably doesn't matter
#[async_trait]
impl DataSource for BinaryEdge {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        trace!("fetching data from binaryedge for: {}", &host);
        let mut tasks = Vec::new();
        let mut results = Vec::new();
        let resp = next_page(self.client.clone(), host.clone(), None).await?;

        // insert subdomains from first page.
        results.extend(resp.subdomains().into_iter());
        let mut page = resp.page;

        loop {
            let host = host.clone();
            let client = self.client.clone();

            if page > 0 && page * resp.pagesize >= resp.total {
                break;
            }

            page += 1;
            tasks.push(tokio::task::spawn(async move {
                next_page(client, host, Some(page)).await
            }));
        }

        for t in tasks {
            let subs = t.await??.subdomains().into_iter();
            results.extend(subs);
        }

        info!("Discovered {} results for: {}", results.len(), &host);
        if !results.is_empty() {
            let _ = tx.send(results).await;
            return Ok(());
        }

        warn!("no results for {} from BinaryEdge", &host);
        Err(VitaError::SourceError("BinaryEdge".into()))
    }
}

async fn next_page(
    client: Client,
    host: Arc<String>,
    page: Option<i32>,
) -> Result<BinaryEdgeResponse> {
    trace!("fetching a page from binaryedge for: {}", &host);
    let uri = BinaryEdge::default().build_url(&host, page);
    let token = match Creds::read_creds() {
        Ok(creds) => creds.token,
        Err(e) => return Err(e),
    };

    let resp = client.get(&uri).header("X-Key", token).send().await?;

    if resp.status().is_success() {
        let be: BinaryEdgeResponse = resp.json().await?;
        return Ok(be);
    }

    info!("binaryedge returned authentication error");
    Err(VitaError::AuthError("BinaryEdge".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    // Tests passed locally, ignoring for now.
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_string());
        let _ = BinaryEdge::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            BinaryEdge::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ))
    }

    #[tokio::test]
    #[ignore]
    async fn handle_auth_error() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        assert!(matches!(
            BinaryEdge::default().run(host, tx).await.err().unwrap(),
            VitaError::AuthError(_)
        ));
    }
}
