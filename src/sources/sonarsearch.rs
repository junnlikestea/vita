use crate::error::Error;
use crate::error::Result;
use crobat::Crobat;
use std::collections::HashSet;
use std::sync::Arc;

pub async fn run(host: Arc<String>) -> Result<HashSet<String>> {
    let mut client = Crobat::new().await;
    let subs = client.get_subs(host.clone()).await?;

    if !subs.is_empty() {
        Ok(subs)
    } else {
        Err(Error::source_error("SonarSearch", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_results() {
        let host = Arc::new("hackerone.com".to_owned());
        let results = run(host).await.unwrap();
        assert!(!results.is_empty());
    }

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
