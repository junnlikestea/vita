use serde::Deserialize;
use std::collections::HashSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Deserialize, Hash, PartialEq, Debug, Eq)]
struct CrtshResult {
    name_value: String,
}

fn build_url(host: &str) -> String {
    format!("https://crt.sh/?q=%.{}&output=json", host)
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    let mut results: HashSet<String> = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<Vec<CrtshResult>> = surf::get(uri).recv_json().await?;

    match resp {
        Some(data) => data
            .into_iter()
            .map(|s| results.insert(s.name_value))
            .for_each(drop),
        None => eprintln!("Crtsh couldn't find results for:{}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://crt.sh/?q=%.hackerone.com&output=json";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    #[async_test]
    async fn returns_results() {
        let host = "hackerone.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() > 5);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = "anVubmxpa2VzdGVh.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
