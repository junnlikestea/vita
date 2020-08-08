use crate::error::{Error, Result};
use crate::IntoSubdomain;
use dotenv::dotenv;
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, PartialEq)]
struct Creds {
    app_id: String,
    app_secret: String,
}

impl Creds {
    pub fn read_creds() -> Result<Self> {
        dotenv().ok();
        let app_id = env::var("FB_APP_ID");
        let app_secret = env::var("FB_APP_SECRET");

        if app_id.is_ok() && app_secret.is_ok() {
            Ok(Self {
                app_id: app_id?,
                app_secret: app_secret?,
            })
        } else {
            Err(Error::key_error(
                "Facebook",
                &["FB_APP_ID", "FB_APP_SECRET"],
            ))
        }
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

        let resp: Option<AuthResp> = reqwest::get(&auth_url).await?.json().await?;

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
    let access_token = match Creds::read_creds() {
        Ok(c) => c.authenticate().await?,
        Err(_) => {
            return Err(Error::key_error(
                "Facebook",
                &["FB_APP_ID", "FB_APP_SECRET"],
            ))
        }
    };

    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(4))
        .build()?;
    let uri = build_url(&host, &access_token);
    let resp: Option<FacebookResult> = client.get(&uri).send().await?.json().await?;

    match resp {
        Some(data) => Ok(data.subdomains()),
        None => Err(Error::source_error("Facebook", host)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // checks if we can fetch the credentials from an .env file.
    #[ignore]
    #[test]
    fn get_creds() {
        dotenv().ok();
        let app_id = env::var("FB_APP_ID").unwrap();
        let app_secret = env::var("FB_APP_SECRET").unwrap();
        let creds: Creds = Creds { app_id, app_secret };
        assert_eq!(creds, Creds::read_creds().unwrap());
    }

    #[test]
    fn get_no_creds() {
        let creds = Creds::read_creds();
        let e = creds.unwrap_err();
        let correct_msg = r#"Couldn't read ["FB_APP_ID", "FB_APP_SECRET"] for Facebook. Check if you have them set."#;
        assert_eq!(e.to_string(), correct_msg);
    }

    // Checks if we can authenticate with Facebook.
    #[ignore]
    #[tokio::test]
    async fn auth() {
        let token = Creds::read_creds().unwrap().authenticate().await.unwrap();
        assert!(token.len() > 1);
    }

    // Checks to see if the run function returns subdomains
    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    // Checks that if we get no results that we just return an error.
    // test is ignored by default to preserve limits
    #[ignore]
    #[tokio::test]
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
