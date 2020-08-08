use crate::error::{Error, Result};
use crate::IntoSubdomain;
use base64::write::EncoderWriter as Base64Encoder;
use dotenv::dotenv;
use reqwest::header::ACCEPT;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

struct Creds {
    key: String,
    secret: String,
}

impl Creds {
    fn from_env() -> Result<Self> {
        dotenv().ok();
        let key = env::var("PASSIVETOTAL_KEY");
        let secret = env::var("PASSIVETOTAL_SECRET");
        if key.is_ok() && secret.is_ok() {
            Ok(Self {
                key: key?,
                secret: secret?,
            })
        } else {
            Err(Error::key_error(
                "PassiveTotal",
                &["PASSIVETOTAL_KEY", "PASSIVETOTAL_SECRET"],
            ))
        }
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

#[derive(Debug, Deserialize)]
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
    trace!("fetching data from passivetotal for: {}", &host);
    let creds = match Creds::from_env() {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    let uri = build_url();
    let query = Query::new(&host);

    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(4))
        .build()?;
    let resp: PassiveTotalResult = client
        .get(&uri)
        .basic_auth(&creds.key, Some(&creds.secret))
        .header(ACCEPT, "application/json")
        .json(&query)
        .send()
        .await?
        .json()
        .await?;

    debug!("passivetotal response: {:?}", &resp);
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

    // Checks to see if the run function returns subdomains
    #[tokio::test]
    #[ignore]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    #[ignore]
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
