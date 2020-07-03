use crate::ResponseData;
use crate::Result;
use dotenv::dotenv;
use http_types::headers;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

#[derive(Deserialize)]
struct SpyseResult {
    data: SpyseItem,
}

#[derive(Deserialize)]
struct SpyseItem {
    items: Vec<Subdomain>,
}

#[derive(Deserialize)]
struct Subdomain {
    name: String,
}

impl ResponseData for SpyseResult {
    fn subdomains(&self, map: &mut HashSet<String>) {
        self.data
            .items
            .iter()
            .map(|i| map.insert(i.name.to_owned()))
            .for_each(drop);
    }
}

fn build_url(host: &str) -> String {
    format!(
        "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain={}",
        host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    // should this process be done with lazy_static macro? otherwise we would be
    // creatng this for every call to run
    //
    // TODO:// handle pagnation?
    dotenv().ok();
    let api_token = env::var("SPYSE_TOKEN")
        .expect("SPYSE_TOKEN must be set in order to use Spyse as a data source");
    let uri = build_url(&host);
    let mut results = HashSet::new();
    let resp: Option<SpyseResult> = surf::get(uri)
        .set_header(headers::ACCEPT, "application/json")
        .set_header(headers::AUTHORIZATION, format!("Bearer {}", api_token))
        .recv_json()
        .await?;

    match resp {
        Some(d) => d.subdomains(&mut results),
        None => eprintln!("Spyse coudln't find any results for: {}", &host),
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
            "https://api.spyse.com/v3/data/domain/subdomain?limit=100&domain=hackerone.com";
        assert_eq!(correct_uri, build_url("hackerone.com"));
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 3);
    }

    #[ignore]
    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() < 1);
    }
}
