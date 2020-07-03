use crate::ResponseData;
use crate::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
struct DnsResult {
    #[serde(rename = "FDNS_A")]
    items: Option<Vec<String>>,
}

impl ResponseData for DnsResult {
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.items
            .iter()
            .flatten()
            .map(|s| map.insert(s.split(',').collect::<Vec<&str>>()[1].to_owned()))
            .for_each(drop);
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TlsResult {
    #[serde(rename = "Results")]
    items: Option<Vec<String>>,
}

impl ResponseData for TlsResult {
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.items
            .iter()
            .flatten()
            .map(|s| map.insert(s.split(',').collect::<Vec<&str>>()[2].to_owned()))
            .for_each(drop);
    }
}

fn build_url(host: &str, dns: bool) -> String {
    if dns {
        format!("http://dns.bufferover.run/dns?q={}", host)
    } else {
        format!("http://tls.bufferover.run/dns?q={}", host)
    }
}

// query the api returns unique results
pub async fn run(host: Arc<String>, dns: bool) -> Result<HashSet<String>> {
    let uri = build_url(&host, dns);
    let mut results = HashSet::new();

    // check if we are fetching results for `dns.bufferover.run` or tls
    if dns {
        let resp: DnsResult = surf::get(uri).recv_json().await?;
        resp.subdomains(&mut results);
    } else {
        let resp: TlsResult = surf::get(uri).recv_json().await?;
        resp.subdomains(&mut results);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn dns_url() {
        let correct_uri = "http://dns.bufferover.run/dns?q=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com", true));
    }

    #[test]
    fn tls_url() {
        let correct_uri = "http://tls.bufferover.run/dns?q=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com", false));
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let results = run(host, true).await.unwrap();
        assert!(results.len() < 1);
    }

    #[async_test]
    async fn dns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host, true).await.unwrap();
        assert!(results.len() > 1);
    }

    #[async_test]
    async fn tls_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host, false).await.unwrap();
        assert!(results.len() > 1);
    }
}
