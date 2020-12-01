use crate::error::Result;
use crate::sources::{
    alienvault::AlienVault, anubisdb::AnubisDB, binaryedge::BinaryEdge, c99::C99,
    certspotter::CertSpotter, chaos::Chaos, crtsh::Crtsh, facebook::Facebook,
    hackertarget::HackerTarget, intelx::Intelx, passivetotal::PassiveTotal,
    securitytrails::SecurityTrails, sonarsearch::SonarSearch, spyse::Spyse, sublister::Sublister,
    threatcrowd::ThreatCrowd, threatminer::ThreatMiner, urlscan::UrlScan, virustotal::VirusTotal,
    wayback::Wayback,
};
use crate::{client, DataSource};

use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use futures_core::stream::Stream;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;

const CHAN_SIZE: usize = 255;

// Configuration options for the `Runner`
struct Config {
    timeout: u64,
    // The maximum number of conurrent tasks
    concurrency: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: 15,
            concurrency: 200,
        }
    }
}

// The `Runner` is responsible for collecting data from all the sources.
pub struct Runner {
    client: Client,
    sources: Vec<Arc<dyn DataSource>>,
    config: Config,
}

impl Default for Runner {
    fn default() -> Self {
        let config = Config::default();
        Self {
            client: client!(config.timeout, config.timeout),
            sources: Vec::new(),
            config,
        }
    }
}

impl Runner {
    /// Sets the limit of concurrent tasks
    pub fn concurrency(mut self, limit: usize) -> Self {
        self.config.concurrency = limit;
        self
    }

    /// Sets the request timeout
    pub fn timeout(mut self, duration: u64) -> Self {
        self.config.timeout = duration;
        self
    }

    /// Sets the sources to be all those which do not require an api key to use.
    pub fn free_sources(mut self) -> Self {
        // Client uses Arc internally so we're just cloning pointers
        let free: Vec<Arc<dyn DataSource>> = vec![
            Arc::new(AnubisDB::new(self.client.clone())),
            Arc::new(AlienVault::new(self.client.clone())),
            Arc::new(CertSpotter::new(self.client.clone())),
            Arc::new(Crtsh::new(self.client.clone())),
            Arc::new(ThreatCrowd::new(self.client.clone())),
            Arc::new(UrlScan::new(self.client.clone())),
            Arc::new(VirusTotal::new(self.client.clone())),
            Arc::new(ThreatMiner::new(self.client.clone())),
            Arc::new(Sublister::new(self.client.clone())),
            Arc::new(Wayback::new(self.client.clone())),
            Arc::new(HackerTarget::new(self.client.clone())),
            Arc::new(SonarSearch::new(self.client.clone())),
        ];

        self.sources.extend(free.into_iter());
        self
    }

    /// Sets the sources to include api keys in addition to the free sources.
    pub fn all_sources(mut self) -> Self {
        let all: Vec<Arc<dyn DataSource>> = vec![
            Arc::new(AnubisDB::new(self.client.clone())),
            Arc::new(AlienVault::new(self.client.clone())),
            Arc::new(CertSpotter::new(self.client.clone())),
            Arc::new(Crtsh::new(self.client.clone())),
            Arc::new(ThreatCrowd::new(self.client.clone())),
            Arc::new(UrlScan::new(self.client.clone())),
            Arc::new(VirusTotal::new(self.client.clone())),
            Arc::new(ThreatMiner::new(self.client.clone())),
            Arc::new(Sublister::new(self.client.clone())),
            Arc::new(SecurityTrails::new(self.client.clone())),
            Arc::new(Wayback::new(self.client.clone())),
            Arc::new(HackerTarget::new(self.client.clone())),
            Arc::new(SonarSearch::new(self.client.clone())),
            Arc::new(BinaryEdge::new(self.client.clone())),
            Arc::new(PassiveTotal::new(self.client.clone())),
            Arc::new(Facebook::new(self.client.clone())),
            Arc::new(Spyse::new(self.client.clone())),
            Arc::new(C99::new(self.client.clone())),
            Arc::new(Intelx::new(self.client.clone())),
            Arc::new(Chaos::new(self.client.clone())),
        ];

        self.sources.extend(all.into_iter());
        self
    }

    /// Fetches data from the sources concurrently
    pub async fn run<T>(self, hosts: T) -> Result<impl Stream<Item = Vec<String>>>
    where
        T: IntoIterator<Item = String>,
    {
        let (tx, rx) = mpsc::channel::<Vec<String>>(CHAN_SIZE);
        let sources = Arc::new(self.sources);

        let mut futures = FuturesUnordered::new();
        let mut outs = Vec::new();
        for host in hosts.into_iter() {
            let host = Arc::new(host);

            if futures.len() >= self.config.concurrency {
                outs.push(futures.next().await.unwrap());
            }

            for source in sources.iter() {
                let source = Arc::clone(source);
                let host = Arc::clone(&host);
                let tx = tx.clone();
                futures.push(tokio::spawn(async move { source.run(host, tx).await }));
            }
        }

        // Get the remaining futures
        while let Some(res) = futures.next().await {
            if let Err(e) = res {
                warn!("got error {} when trying to recv remaining futures", e)
            }
        }

        // explicitly drop the remaning sender
        drop(tx);
        Ok(rx)
    }
}
