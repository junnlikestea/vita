use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct Subdomain {
    hostname: String,
}

#[derive(Deserialize)]
struct AlienvaultResult {
    passive_dns: Vec<Subdomain>,
    count: i32,
}

fn build_url(host: &str) -> String {
    format!(
        "https://otx.alienvault.com/api/v1/indicators/domain/{}/passive_dns",
        host
    )
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: AlienvaultResult = surf::get(uri).recv_json().await?;

    if resp.count > 0 {
        resp.passive_dns
            .into_iter()
            .map(|s| results.insert(s.hostname))
            .for_each(drop);
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
        let results = run("hackerone.com".to_owned()).await.unwrap();
        for r in results.iter() {
            println!("{}", r);
        }
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "anVubmxpa2VzdGVh.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() == 0);
    }
}
