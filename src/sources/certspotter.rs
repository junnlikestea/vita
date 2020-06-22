use async_std::{prelude::*, task};
use serde::Deserialize;
use std::collections::HashSet;

// this is replicated in manyt places
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Deserialize, Hash, PartialEq, Eq, Debug)]
struct CertSpotterResult {
    dns_names: Vec<String>,
}

fn build_url(host: &str) -> String {
    format!(
        "https://api.certspotter.com/v1/issuances?domain={}\
        &include_subdomains=true&expand=dns_names",
        host
    )
}

pub async fn run(host: &str) -> Result<HashSet<String>> {
    let uri = build_url(host);
    let resp: HashSet<CertSpotterResult> = surf::get(uri).recv_json().await?;
    let results: HashSet<String> = resp
        .into_iter()
        .flat_map(|s| s.dns_names.into_iter())
        .collect();

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
        let correct_uri = "https://api.certspotter.com/v1/issuances?domain=hackerone.com\
        &include_subdomains=true&expand=dns_names";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }
}
