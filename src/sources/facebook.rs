use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
struct Credentials {
    app_id: String,
    app_secret: String,
}

impl Credentials {
    pub fn from_env() -> Self {
        dotenv().ok();
        let app_id = env::var("FB_APP_ID").expect("FB_APP_ID must be set");
        let app_secret = env::var("FB_APP_SECRET").expect("FB_APP_SECRET must be set");
        Credentials { app_id, app_secret }
    }

    pub async fn authenticate(&self) -> Result<String> {
        // created a struct because deserializing into a serde_json::Value
        // was returning the access token with quotation marks"tokeninhere"
        // but wasn't doing that as a struct.
        #[derive(Deserialize)]
        struct AuthResp {
            access_token: String,
        }

        let auth_url = format!(
            "https://graph.facebook.com/oauth/access_token?client_id={}\
            &client_secret={}&grant_type=client_credentials",
            self.app_id, self.app_secret
        );

        let resp: Option<AuthResp> = surf::get(auth_url).recv_json().await?;

        if let Some(r) = resp {
            Ok(r.access_token)
        } else {
            Err(Error::auth_error("Facebook"))
        }
    }
}

#[derive(Deserialize, Debug)]
struct Subdomains {
    domains: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct FacebookResult {
    data: Vec<Subdomains>,
}

impl IntoSubdomain for FacebookResult {
    fn subdomains(&self) -> HashSet<String> {
        self.data
            .iter()
            .flat_map(|s| s.domains.iter())
            .map(|r| r.to_owned())
            .collect()
    }
}

fn build_url(host: &str, token: &str) -> String {
    format!(
        "https://graph.facebook.com/certificates?fields=domains&access_token={}&query=*.{}",
        token, host
    )
}

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let access_token = Credentials::from_env().authenticate().await?;
    let uri = build_url(&host, &access_token);
    let resp: Option<FacebookResult> = surf::get(uri).recv_json().await?;

    match resp {
        Some(data) => Ok(data.subdomains()),
        None => Err(Error::source_error("Facebook", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    // checks if we can fetch the credentials from an .env file.
    #[ignore]
    #[test]
    fn get_creds() {
        dotenv().ok();
        let app_id = env::var("FB_APP_ID").expect("FB_APP_ID must be set");
        let app_secret = env::var("FB_APP_SECRET").expect("FB_APP_SECRET must be set");
        let creds: Credentials = Credentials { app_id, app_secret };
        assert_eq!(creds, Credentials::from_env());
    }

    // Checks if we can authenticate with Facebook.
    #[async_test]
    #[ignore]
    async fn auth() {
        let token = Credentials::from_env().authenticate().await.unwrap();
        assert!(token.len() > 1);
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[async_test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(results.len() > 3);
    }

    // Checks that if we get no results that we just return an error.
    // test is ignored by default to preserve limits
    #[ignore]
    #[async_test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_string());
        let res = run(host).await;
        let e = res.unwrap_err();
        assert_eq!(
            e.to_string(),
            "Facebook couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
