use crate::ResponseData;
use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct BufferOverResult {
    #[serde(rename = "FDNS_A")]
    items: Option<Vec<String>>,
}

fn build_url(host: &str) -> String {
    format!("http://dns.bufferover.run/dns?q={}", host)
}

impl ResponseData for Vec<String> {
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.iter()
            .map(|s| map.insert(s.split(',').collect::<Vec<&str>>()[1].to_owned()))
            .for_each(drop);
    }
}

// query the api returns unique results
pub async fn run(host: String) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let mut results = HashSet::new();
    let resp: BufferOverResult = surf::get(uri).recv_json().await?;

    match resp.items {
        Some(data) => data.subdomains(&mut results),
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
