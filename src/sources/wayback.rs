use crate::ResponseData;
use crate::Result;
use serde_json::value::Value;
use std::collections::HashSet;
use std::sync::Arc;
use url::Url;

struct WaybackResult {
    data: Value,
}

impl ResponseData for WaybackResult {
    fn subdomains(&self, map: &mut HashSet<String>) {
        let arr = self.data.as_array().unwrap();
        let vecs: Vec<&str> = arr.iter().map(|s| s[0].as_str().unwrap()).collect();
        for v in vecs.into_iter() {
            match Url::parse(v) {
                Ok(u) => map.insert(u.host_str().unwrap_or("").into()),
                _ => false,
            };
        }
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://web.archive.org/cdx/search/cdx?url=*.{}/*&output=json\
    &fl=original&collapse=urlkey&limit=100000",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<Value> = surf::get(uri).recv_json().await?;

    match resp {
        Some(data) => WaybackResult { data }.subdomains(&mut results),
        None => eprintln!("Wayback Machine couldn't find any results for: {}", &host),
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[test]
    fn url_builder() {
        let correct_uri =
            "https://web.archive.org/cdx/search/cdx?url=*.hackerone.com/*&output=json\
    &fl=original&collapse=urlkey&limit=100000";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    //
    #[ignore] // hangs forever on windows for some reasons?
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 0);
    }
}
