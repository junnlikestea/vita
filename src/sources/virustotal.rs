use crate::error::{Error, Result};
use crate::IntoSubdomain;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Deserialize)]
struct Subdomain {
    id: String,
}

#[derive(Deserialize)]
struct VirustotalResult {
    data: Option<Vec<Subdomain>>,
}

impl IntoSubdomain for VirustotalResult {
    fn subdomains(&self) -> Vec<String> {
        self.data
            .iter()
            .flatten()
            .map(|s| s.id.to_owned())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    // TODO: can we gather the subdomains using:
    // Handle pagenation
    // https://www.virustotal.com/vtapi/v2/domain/report
    format!(
        "https://www.virustotal.com/ui/domains/{}/subdomains?limit=40",
        host
    )
}

pub async fn run(client: Client, host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    trace!("fetching data from virustotal for: {}", &host);
    let uri = build_url(&host);
    let resp: VirustotalResult = client.get(&uri).send().await?.json().await?;
    let subdomains = resp.subdomains();

    if !subdomains.is_empty() {
        info!("Discovered {} results for {}", &subdomains.len(), &host);
        if let Err(e) = sender.send(subdomains).await {
            error!("got error {} when sending to channel", e)
        }
        Ok(())
    } else {
        warn!("No results found for {}", &host);
        Err(Error::source_error("VirusTotal", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client;
    use std::time::Duration;
    use tokio::sync::mpsc::channel;

    // IGNORE by default since we have limited api calls.
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let client = client!();
        let mut results = Vec::new();
        run(client, host, tx).await;
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let client = client!();
        let res = run(client, host, tx).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "VirusTotal couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
