use serde::Deserialize;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>; // 4

#[derive(Deserialize)]
struct BufferOverResult {
    #[serde(rename = "FDNS_A")]
    subdomains: Option<Vec<String>>,
}

fn build_url(host: &str) -> String {
    format!("http://dns.bufferover.run/dns?q={}", host)
}

// query the api returns unique results
pub async fn run(host: String) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let mut results = HashSet::new();
    let BufferOverResult { subdomains } = surf::get(uri).recv_json().await?;

    match subdomains {
        Some(data) => {
            data.into_iter()
                .map(|s| results.insert(s.split(',').collect::<Vec<&str>>()[1].to_owned()))
                .for_each(drop);
        }

        None => println!("Bufferover couldn't find results for:{}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "http://dns.bufferover.run/dns?q=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "anVubmxpa2VzdGVh.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
