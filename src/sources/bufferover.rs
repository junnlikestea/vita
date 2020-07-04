use crate::IntoSubdomain;
use crate::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::{error::Error, fmt};

#[derive(Serialize, Deserialize, Debug)]
struct DnsResult {
    #[serde(rename = "FDNS_A")]
    items: Option<Vec<String>>,
}

impl IntoSubdomain for DnsResult {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .iter()
            .flatten()
            .map(|s| s.split(',').collect::<Vec<&str>>()[1].to_owned())
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TlsResult {
    #[serde(rename = "Results")]
    items: Option<Vec<String>>,
}

impl IntoSubdomain for TlsResult {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .iter()
            .flatten()
            .map(|s| s.split(',').collect::<Vec<&str>>()[2].to_owned())
            .collect()
    }
}

#[derive(Debug)]
struct BufferOverError {
    host: Arc<String>,
}

impl BufferOverError {
    fn new(host: Arc<String>) -> Self {
        Self { host }
    }
}

impl Error for BufferOverError {}

impl fmt::Display for BufferOverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BufferOver couldn't find any results for: {}", self.host)
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

    if dns {
        let resp: DnsResult = surf::get(uri).recv_json().await?;
        let subdomains = resp.subdomains();

        if !subdomains.is_empty() {
            Ok(subdomains)
        } else {
            Err(Box::new(BufferOverError::new(host)))
        }
    } else {
        let resp: TlsResult = surf::get(uri).recv_json().await?;
        let subdomains = resp.subdomains();

        if !subdomains.is_empty() {
            Ok(subdomains)
        } else {
            Err(Box::new(BufferOverError::new(host)))
        }
    }
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

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host, true).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "BufferOver couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
