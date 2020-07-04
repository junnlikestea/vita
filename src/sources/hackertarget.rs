use crate::IntoSubdomain;
use crate::Result;
use std::collections::HashSet;
use std::sync::Arc;
const API_ERROR: &str = "error check your search parameter";

struct HackerTarget {
    items: String,
}

impl IntoSubdomain for HackerTarget {
    fn subdomains(&self) -> HashSet<String> {
        self.items
            .lines()
            .map(|s| s.split(',').collect::<Vec<&str>>()[0].to_owned())
            .collect()
    }
}

fn build_url(host: &str) -> String {
    format!("https://api.hackertarget.com/hostsearch/?q={}", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let uri = build_url(&host);
    let mut results = HashSet::new();
    let resp: String = surf::get(uri).recv_string().await?;

    if resp != API_ERROR {
        match Some(resp) {
            Some(items) => return Ok(HackerTarget { items }.subdomains()),
            None => eprintln!("HackerTarget, couldn't find results for:{}", &host),
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 3);
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2V0ZWE.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
