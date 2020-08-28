use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde_json::value::Value;
use std::collections::HashSet;
use std::sync::Arc;
use url::Url;

struct WaybackResult {
    data: Value,
}

impl WaybackResult {
    fn new(data: Value) -> Self {
        Self { data }
    }
}

//TODO: this could be cleaned up, to avoid creating the extra vec `vecs`
impl IntoSubdomain for WaybackResult {
    fn subdomains(&self) -> HashSet<String> {
        let arr = self.data.as_array().unwrap();
        let vecs: Vec<&str> = arr.iter().map(|s| s[0].as_str().unwrap()).collect();
        vecs.into_iter()
            .filter_map(|a| Url::parse(a).ok())
            .map(|u| u.host_str().unwrap().into())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://web.archive.org/cdx/search/cdx?url=*.{}/*&output=json\
    &fl=original&collapse=urlkey&limit=100000",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from wayback for: {}", &host);
    let uri = build_url(&host);
    let resp: Option<Value> = client.get(&uri).send().await?.json().await?;

    match resp {
        Some(data) => {
            let subdomains = WaybackResult::new(data).subdomains();

            if !subdomains.is_empty() {
                info!("Discovered {} results for: {}", &subdomains.len(), &host);
                Ok(subdomains)
            } else {
                warn!("No results found for: {}", &host);
                Err(Error::source_error("Wayback Machine", host))
            }
        }

        None => Err(Error::source_error("Wayback Machine", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://web.archive.org/cdx/search/cdx?url=*.hackerone.com/*&output=json\
    &fl=original&collapse=urlkey&limit=100000";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    #[ignore] // hangs forever on windows for some reasons?
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Wayback Machine couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
