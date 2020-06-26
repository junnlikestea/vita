use serde_json::value::Value;
use std::collections::HashSet;
use url::Url;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn build_url(host: &str) -> String {
    format!(
        "https://web.archive.org/cdx/search/cdx?url=*.{}/*&output=json\
    &fl=original&collapse=urlkey&limit=100000&_=1547318148315",
        host
    )
}

fn parse_result(result: Value, map: &mut HashSet<String>) {
    let arr = result.as_array().unwrap();
    let vecs: Vec<&str> = arr.into_iter().map(|s| s[0].as_str().unwrap()).collect();
    for v in vecs.into_iter() {
        match Url::parse(v) {
            Ok(u) => map.insert(u.host_str().unwrap_or("").into()),
            _ => false,
        };
    }
}

pub async fn run(host: String) -> Result<HashSet<String>> {
    let mut results = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<Value> = surf::get(uri).recv_json().await?;
    match resp {
        Some(d) => parse_result(d, &mut results),
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
    &fl=original&collapse=urlkey&limit=100000&_=1547318148315";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[async_test]
    async fn returns_results() {
        let results = run("hackerone.com".to_owned()).await.unwrap();
        assert!(results.len() > 0);
    }
}
