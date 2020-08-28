use crate::error::Error;
use crate::error::Result;
use crobat::Crobat;
use std::collections::HashSet;
use std::sync::Arc;

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut client = Crobat::new().await;
    let subdomains = client.get_subs(host.clone()).await?;

    if !subdomains.is_empty() {
        info!("Discovered {} results for: {}", &subdomains.len(), &host);
        Ok(subdomains)
    } else {
        warn!("No results for: {}", &host);
        Err(Error::source_error("SonarSearch", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = run(host).await;
        let e = results.unwrap_err();
        assert_eq!(
            e.to_string(),
            "SonarSearch couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
