use crate::Result;
use serde_json::value::Value;
use std::collections::HashSet;

fn build_url(host: &str) -> String {
    format!("https://jldc.me/anubis/subdomains/{}", host)
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<Value> = surf::get(uri).recv_json().await?;

    match resp {
        Some(d) => d
            .as_array()
            .unwrap()
            .iter()
            .map(|s| results.insert(s.as_str().unwrap().into()))
            .for_each(drop),
        None => eprintln!("AnubisDB couldn't find any results for: {}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://jldc.me/anubis/subdomains/hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let results = run("hackerone.com".to_owned()).await.unwrap();
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "anVubmxpa2VzdGVh.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() == 0);
    }
}
