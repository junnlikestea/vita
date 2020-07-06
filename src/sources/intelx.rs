use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

struct Creds {
    url: String,
    api_key: String,
}

impl Creds {
    fn from_env() -> Self {
        dotenv().ok();
        let api_key = env::var("INTELX_KEY")
            .expect("INTELX_KEY must be set in order to use Intelx as a data source");
        let url = env::var("INTELX_URL")
            .expect("INTELX_URL must be set in order to use Intelx as a data source");
        Self { url, api_key }
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
    fn subdomains(&self) -> HashSet<String> {
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

async fn get_searchid(host: Arc<String>) -> Result<String> {
    let creds = Creds::from_env();
    let query_uri = build_url(&creds.url, &creds.api_key, true, None);
    let body = Query::new(host.to_string());
    let search_id: SearchId = surf::post(query_uri).body_json(&body)?.recv_json().await?;

    Ok(search_id.id)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let creds = Creds::from_env();
    let search_id = get_searchid(host.clone()).await?;
    let uri = build_url(&creds.url, &creds.api_key, false, Some(&search_id));
    let resp: IntelxResults = surf::get(uri).recv_json().await?;
    let subdomains = resp.subdomains();

    if !subdomains.is_empty() {
        Ok(subdomains)
    } else {
        Err(Error::source_error("Intelx", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[async_test]
    #[ignore]
    async fn search_id() {
        let host = Arc::new("hackerone.com".to_owned());
        let id = get_searchid(host).await.unwrap();
        println!("{}", &id);
        assert!(id.len() > 0)
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        for r in results.iter() {
            println!("{}", r);
        }
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
            "Intelx couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
