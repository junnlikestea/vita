use crate::error::{Result, VitaError};
use crate::{DataSource, QUEUE_SIZE};
use async_trait::async_trait;
use crobat::Crobat;
use futures::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

#[derive(Default, Clone)]
pub struct SonarSearch {
    client: Client,
}

impl SonarSearch {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl DataSource for SonarSearch {
    async fn run(&self, host: Arc<String>, mut tx: Sender<Vec<String>>) -> Result<()> {
        let mut results = Vec::with_capacity(QUEUE_SIZE);
        let mut client = Crobat::new().await;
        let mut subs = client.get_subs(host.clone()).await?;

        while let Some(r) = subs.next().await {
            let domain = r.map(|d| d.domain).map_err(|_| VitaError::CrobatError)?;
            results.push(domain);

            if results.len() == QUEUE_SIZE {
                debug!("sonarsearch queue is full, sending across channel",);
                let mut tx = tx.clone();
                let _ = tx.send(results.drain(..).collect()).await;
            }
        }

        if !results.is_empty() {
            info!(
                "draining {} remaining items from sonarsearch queue",
                results.len()
            );
            let _ = tx.send(results.drain(..).collect()).await;
        }

        Err(VitaError::SourceError("SonarSearch".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::matches;
    use tokio::sync::mpsc::channel;

    #[ignore]
    #[tokio::test]
    async fn returns_results() {
        let (tx, mut rx) = channel(1);
        let host = Arc::new("hackerone.com".to_owned());
        let _ = SonarSearch::default().run(host, tx).await;
        let mut results = Vec::new();
        for r in rx.recv().await {
            results.extend(r)
        }
        assert!(!results.is_empty());
    }

    #[ignore]
    #[tokio::test]
    async fn handle_no_results() {
        let (tx, _rx) = channel(1);
        let host = Arc::new("anVubmxpa2VzdGVh.com".to_owned());
        assert!(matches!(
            SonarSearch::default().run(host, tx).await.err().unwrap(),
            VitaError::SourceError(_)
        ));
    }
}
