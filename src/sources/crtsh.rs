use crate::ResponseData;
use crate::Result;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Deserialize, Hash, PartialEq, Debug, Eq)]
struct CrtshResult {
    name_value: String,
}

impl ResponseData for Vec<CrtshResult> {
    //todo: is there any need to allocate a new string each time? couldn't
    // we just pass around references to the string inside the crtshresult struct?
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.iter()
            .map(|s| map.insert(s.name_value.to_owned()))
            .for_each(drop);
    }
}

fn build_url(host: &str) -> String {
    format!("https://crt.sh/?q=%.{}&output=json", host)
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut results: HashSet<String> = HashSet::new();
    let uri = build_url(&host);
    let resp: Option<Vec<CrtshResult>> = surf::get(uri).recv_json().await?;

    match resp {
        Some(data) => data.subdomains(&mut results),
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

    #[ignore]
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 5);
    }

    #[ignore] // tests passing locally but failing on linux ci?
    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
