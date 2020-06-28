use crate::Result;
use dotenv::dotenv;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;

#[derive(Deserialize)]
struct SecTrailsResult {
    subdomains: Vec<String>,
}

fn build_url(host: &str) -> String {
    format!(
        "https://api.securitytrails.com/v1/domain/{}/subdomains",
        host
    )
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    dotenv().ok();
    let api_key = env::var("SECURITY_TRAILS_KEY")
        .expect("SECURITY_TRAILS_KEY must be set to use Security Trails API");
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<SecTrailsResult> = surf::get(uri)
        .set_header("apikey", api_key)
        .recv_json()
        .await?;
    // secuirity trails doesn't add the host name to the result.
    // so api.hackerone.com will just be api in the results.
    // we will add the hostname manually to result.
    match resp {
        Some(d) => d
            .subdomains
            .into_iter()
            .map(|s| results.insert(format!("{}.{}", s, &host)))
            .for_each(drop),
        None => eprintln!("Security Trails couldn't find any results for: {}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.securitytrails.com/v1/domain/hackerone.com/subdomains";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[async_test]
    async fn returns_results() {
        let results = run("hackerone.com".to_owned()).await.unwrap();
        assert!(results.len() > 0);
    }

    #[ignore]
    #[async_test]
    async fn handle_no_results() {
        let host = "hdsad.com".to_owned();
        let results = run(host).await.unwrap();
        assert!(results.len() == 0);
    }
}
