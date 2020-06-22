use async_std::{prelude::*, task};
use serde::Deserialize;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>; // 4

#[derive(Deserialize)]
struct BufferOverResult {
    #[serde(rename = "FDNS_A")]
    subdomains: Vec<String>,
}

fn build_url(host: &str) -> String {
    format!("http://dns.bufferover.run/dns?q={}", host)
}

// query the api returns unique results
pub async fn run(host: &str) -> Result<HashSet<String>> {
    let uri = build_url(host);
    let mut results = HashSet::new();
    let BufferOverResult { subdomains } = surf::get(uri).recv_json().await?;

    // do we need to user into_iter here? since we want to take ownerships of s anyway
    for s in subdomains.into_iter() {
        let sub = s.split(",").collect::<Vec<&str>>()[1].to_owned();
        results.insert(sub);
    }

    Ok(results)
}

// query the api for multiple hosts at a time.
pub async fn run_all(hosts: Vec<&str>) -> Result<HashSet<String>> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_builder() {
        let correct_uri = "http://dns.bufferover.run/dns?q=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }
}
