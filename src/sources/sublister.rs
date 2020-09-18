use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde_json::value::Value;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

struct SublisterResult {
    items: Vec<Value>,
}

impl SublisterResult {
    fn new(items: Vec<Value>) -> Self {
        SublisterResult { items }
    }
}

//TODO: can this just be collected without the map?
impl IntoSubdomain for SublisterResult {
    fn subdomains(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://api.sublist3r.com/search.php?domain={}", host)
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from sublister for: {}", &host);
    let uri = build_url(&host);
    let resp: Option<Value> = client.get(&uri).send().await?.json().await?;

    debug!("sublister resp: {:?}", &resp);
    match resp {
        Some(d) => {
            let subdomains = SublisterResult::new(d.as_array().unwrap().to_owned()).subdomains();

            if !subdomains.is_empty() {
                info!("Discovered {} results for {}", &subdomains.len(), &host);
                let _ = sender.send(subdomains).await?;
                Ok(())
            } else {
                warn!("No results for: {}", &host);
                Err(Error::source_error("Sublist3r", host))
            }
        }

        None => Err(Error::source_error("Sublist3r", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;
    use tokio::sync::mpsc::channel;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.sublist3r.com/search.php?domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[tokio::test]
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
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Sublist3r couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
