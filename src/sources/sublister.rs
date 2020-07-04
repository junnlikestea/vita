use crate::IntoSubdomain;
use crate::Result;
use serde_json::value::Value;
use std::collections::HashSet;
use std::sync::Arc;

struct SublisterResult {
    items: Vec<Value>,
}

fn build_url(host: &str) -> String {
    format!("https://api.sublist3r.com/search.php?domain={}", host)
}

impl IntoSubdomain for SublisterResult {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    }
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<Value> = surf::get(uri).recv_json().await?;
    //TODO: isn't there a more efficient way to do this (complexity wise)
    // not just this source, but multiple sources have unecessary loops.
    match resp {
        Some(d) => {
            //kinda messy
            let data = SublisterResult {
                items: d.as_array().unwrap().to_owned(),
            };

            return Ok(data.subdomains());
        }
        None => eprintln!("Sublist3r couldn't find any results for: {}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri = "https://api.sublist3r.com/search.php?domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 0);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("hdsad.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() == 0);
    }
}
