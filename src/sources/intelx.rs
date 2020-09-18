use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

struct Creds {
    url: String,
    api_key: String,
}

impl Creds {
    fn read_creds() -> Result<Self> {
        dotenv().ok();
        let api_key = env::var("INTELX_KEY");
        let url = env::var("INTELX_URL");

        if api_key.is_ok() && url.is_ok() {
            Ok(Self {
                url: url?,
                api_key: api_key?,
            })
        } else {
            Err(Error::key_error("Intelx", &["INTELX_URL", "INTELX_KEY"]))
        }
    }
}

#[derive(Deserialize, Debug)]
struct SearchId {
    id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Query {
    term: String,
    maxresults: i32,
    media: usize,
    target: usize,
    timeout: i32,
}

impl Query {
    fn new(term: String) -> Self {
        Self {
            term,
            maxresults: 100000,
            media: 0,
            target: 1,
            timeout: 20,
        }
    }
}

#[derive(Deserialize, Debug)]
struct IntelxItem {
    selectorvalue: String,
}

#[derive(Deserialize, Debug)]
struct IntelxResults {
    selectors: Vec<IntelxItem>,
    status: usize,
}

impl IntoSubdomain for IntelxResults {
    fn subdomains(&self) -> Vec<String> {
        self.selectors
            .iter()
            .map(|s| s.selectorvalue.to_owned())
            .collect()
    }
}

// 9df61df0-84f7-4dc7-b34c-8ccfb8646ace
fn build_url(intelx_url: &str, api_key: &str, querying: bool, search_id: Option<&str>) -> String {
    if querying {
        format!("https://{}/phonebook/search?k={}", intelx_url, api_key)
    } else {
        format!(
            "https://{}/phonebook/search/result?k={}&id={}&limit=100000",
            intelx_url,
            api_key,
            search_id.unwrap()
        )
    }
}

async fn get_searchid(client: Client, host: Arc<String>) -> Result<String> {
    trace!("getting intelx searchid");
    let creds = match Creds::read_creds() {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    let query_uri = build_url(&creds.url, &creds.api_key, true, None);
    let body = Query::new(host.to_string());
    let search_id: SearchId = client
        .post(&query_uri)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    debug!("searchid: {:?}", &search_id);
    Ok(search_id.id)
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from intelx for: {}", &host);
    let creds = match Creds::read_creds() {
        Ok(creds) => creds,
        Err(e) => return Err(e),
    };

    let search_id = get_searchid(client.clone(), host.clone()).await?;
    let uri = build_url(&creds.url, &creds.api_key, false, Some(&search_id));
    let resp = client.get(&uri).send().await?;

    if resp.status().is_client_error() {
        warn!("got status: {} for intelx", resp.status().as_str());
        Err(Error::auth_error("intelx"))
    } else {
        let resp: IntelxResults = resp.json().await?;
        let subdomains = resp.subdomains();
        debug!("intelx response: {:?}", &resp);

        if !subdomains.is_empty() {
            info!("Discovered {} results for: {}", &subdomains.len(), &host);
            let _ = sender.send(subdomains).await?;
            Ok(())
        } else {
            warn!("No results for: {}", &host);
            Err(Error::source_error("Intelx", host))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;
    use tokio::sync::mpsc::channel;

    #[tokio::test]
    #[ignore]
    async fn search_id() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let id = get_searchid(client, host).await.unwrap();
        assert!(!id.is_empty())
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let mut results = Vec::new();
        let _ = run(client, host, tx).await;
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
        let client = client!();
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Intelx couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
