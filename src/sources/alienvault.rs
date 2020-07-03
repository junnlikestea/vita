use crate::ResponseData;
use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize)]
struct Subdomain {
    hostname: String,
}

#[derive(Deserialize)]
struct AlienvaultResult {
    passive_dns: Vec<Subdomain>,
    count: i32,
}

impl ResponseData for AlienvaultResult {
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.passive_dns
            .iter()
            .map(|s| map.insert(s.hostname.to_owned()))
            .for_each(drop);
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://otx.alienvault.com/api/v1/indicators/domain/{}/passive_dns",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: AlienvaultResult = surf::get(uri).recv_json().await?;

    if resp.count > 0 {
        resp.subdomains(&mut results);
    } else {
        eprintln!("Alien Vault didn't find any results for: {}", &host);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://otx.alienvault.com/api/v1/indicators/domain/\
        hackerone.com/passive_dns";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_string());
        let results = run(host).await.unwrap();
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let results = run(host).await.unwrap();
        assert!(results.len() == 0);
    }
}
