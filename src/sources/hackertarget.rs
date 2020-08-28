use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use std::collections::HashSet;
use std::sync::Arc;

const API_ERROR: &str = "error check your search parameter";

struct HackerTarget {
    items: String,
}

impl HackerTarget {
    fn new(items: String) -> Self {
        HackerTarget { items }
    }
}

impl IntoSubdomain for HackerTarget {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .lines()
            .map(|s| s.split(',').collect::<Vec<&str>>()[0].to_owned())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://api.hackertarget.com/hostsearch/?q={}", host)
}

pub async fn run(client: Client, host: Arc<String>) -> Result<HashSet<String>> {
    trace!("fetching data from hackertarget for: {}", &host);
    let uri = build_url(&host);
    let resp: String = client.get(&uri).send().await?.text().await?;
    debug!("hackertarget response: {:?}", &resp);

    if resp != API_ERROR {
        let subdomains = HackerTarget::new(resp).subdomains();
        info!("Discovered {} results for: {}", &subdomains.len(), &host);
        Ok(subdomains)
    } else {
        warn!("No results found for: {}", &host);
        Err(Error::source_error("HackerTarget", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let results = run(client, host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "HackerTarget couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
