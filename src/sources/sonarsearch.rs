use crate::error::Error;
use crate::error::Result;
use crobat::Crobat;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub async fn run(host: Arc<String>, mut sender: Sender<Vec<String>>) -> Result<()> {
    let mut client = Crobat::new().await;
    let subdomains = client.get_subs(host.clone()).await?;

    if !subdomains.is_empty() {
        info!("Discovered {} results for: {}", &subdomains.len(), &host);
        let _ = sender.send(subdomains).await?;
        Ok(())
    } else {
        warn!("No results for: {}", &host);
        Err(Error::source_error("SonarSearch", host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::channel;

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = run(host, tx).await.unwrap();
        assert!(!rx.recv().await.unwrap().is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        let results = run(host, tx).await;
        let e = results.unwrap_err();
        assert_eq!(
            e.to_string(),
            "SonarSearch couldn't find any results for: anVubmxpa2VzdGVh.com"
        );
    }
}
