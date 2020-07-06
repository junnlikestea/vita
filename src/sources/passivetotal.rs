use crate::error::{Error, Result};
use crate::IntoSubdomain;
use base64::write::EncoderWriter as Base64Encoder;
use dotenv::dotenv;
use http_types::headers;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::io::Write;
use std::sync::Arc;

struct Creds {
    key: String,
    secret: String,
}

impl Creds {
    fn from_env() -> Self {
        dotenv().ok();
        let key = env::var("PASSIVETOTAL_KEY").expect(
            "PASSIVETOTAL_KEY\
        must be set in order to use PassiveTotal as a data source",
        );
        let secret = env::var("PASSIVETOTAL_SECRET").expect(
            "PASSIVETOTAL_SECRET\
must be set in order to use PassiveTotal as a data source",
        );

        Self { key, secret }
    }
}

#[derive(Serialize)]
struct Query {
    query: String,
}

impl Query {
    fn new(host: &str) -> Self {
        Self {
            query: host.to_string(),
        }
    }
}

#[derive(Deserialize)]
struct PassiveTotalResult {
    success: bool,
    #[serde(rename = "primaryDomain")]
    primary_domain: String,
    subdomains: Vec<String>,
}

impl IntoSubdomain for PassiveTotalResult {
    fn subdomains(&self) -> HashSet<String> {
        self.subdomains
            .iter()
            .map(|s| format!("{}.{}", s, self.primary_domain))
            .collect()
    }
}

fn build_url() -> String {
    "https://api.passivetotal.org/v2/enrichment/subdomains".to_string()
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let creds = Creds::from_env();
    let uri = build_url();
    let basic = basic_auth(&creds.key, Some(&creds.secret));
    let query = Query::new(&host);
    let resp: PassiveTotalResult = surf::get(uri)
        .set_header(headers::AUTHORIZATION, basic)
        .set_header(headers::ACCEPT, "application/json")
        .body_json(&query)?
        .recv_json()
        .await?;

    if resp.success {
        let subdomains = resp.subdomains();

        if !subdomains.is_empty() {
            Ok(subdomains)
        } else {
            Err(Error::source_error("PassiveTotal", host))
        }
    } else {
        Err(Error::source_error("PassiveTotal", host))
    }
}

// A method to create a basic authenticaiton header, because surf doesn't have one :(
// https://docs.rs/reqwest/0.10.6/src/reqwest/async_impl/request.rs.html#196-212
fn basic_auth(username: &str, password: Option<&str>) -> String {
    use std::str;
    let mut header_value = b"Basic ".to_vec();
    {
        let mut encoder = Base64Encoder::new(&mut header_value, base64::STANDARD);
        write!(encoder, "{}:", username).unwrap();
        if let Some(password) = password {
            write!(encoder, "{}", password).unwrap();
        }
    }
    str::from_utf8(&header_value).unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // Checks to see if the run function returns subdomains
    #[async_test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        for r in results.iter() {
            println!("{}", r)
        }
        assert!(!results.is_empty());
    }

    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "PassiveTotal couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
