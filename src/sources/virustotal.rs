use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize)]
struct Subdomain {
    id: String,
}

#[derive(Deserialize)]
struct VirustotalResult {
    data: Option<Vec<Subdomain>>,
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

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut results: HashSet<String> = HashSet::new();
    let uri = build_url(&host);
    let resp: VirustotalResult = surf::get(uri).recv_json().await?;

    match resp.data {
        Some(data) => data
            .into_iter()
            .map(|s| results.insert(s.id))
            .for_each(drop),

        None => eprintln!("VirusTotal couldn't find results for: {}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    // IGNORE by default since we have limited api calls.
    #[async_test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 5);
    }

    #[async_test]
    #[ignore]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
